use redis::{Client, Commands, RedisResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RedissonLock {
    client: Client,
    lock_name: String,
    uuid: String,
    lease_time_ms: u64,
    // Track thread ownership locally
    thread_ownership: Arc<Mutex<HashMap<String, ThreadId>>>,
}

impl RedissonLock {
    pub fn new(redis_url: &str, lock_name: &str, lease_time_ms: u64) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        let uuid = Uuid::new_v4().to_string();
        
        Ok(RedissonLock {
            client,
            lock_name: lock_name.to_string(),
            uuid,
            lease_time_ms,
            thread_ownership: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Check if the current thread holds the lock
    /// This is the main implementation of Redisson's isHeldByCurrentThread
    pub fn is_held_by_current_thread(&self) -> RedisResult<bool> {
        let current_thread_id = thread::current().id();
        
        // First check local thread ownership tracking
        if let Ok(ownership_map) = self.thread_ownership.lock() {
            if let Some(&stored_thread_id) = ownership_map.get(&self.lock_name) {
                if stored_thread_id == current_thread_id {
                    // Verify with Redis that the lock is still valid
                    return self.verify_lock_in_redis();
                }
            }
        }
        
        Ok(false)
    }

    /// Verify the lock exists in Redis and belongs to this client instance
    fn verify_lock_in_redis(&self) -> RedisResult<bool> {
        let mut conn = self.client.get_connection()?;
        let lock_key = format!("lock:{}", self.lock_name);
        
        // Check if the lock exists and contains our UUID
        let lock_value: Option<String> = conn.get(&lock_key)?;
        
        match lock_value {
            Some(value) => {
                // Parse the lock value to check if it contains our UUID
                // Redisson typically stores: "{uuid}:{thread_id}:{reentrant_count}"
                Ok(value.starts_with(&self.uuid))
            }
            None => Ok(false),
        }
    }

    /// Acquire the lock (helper method for testing)
    pub fn try_lock(&self) -> RedisResult<bool> {
        let mut conn = self.client.get_connection()?;
        let lock_key = format!("lock:{}", self.lock_name);
        let current_thread_id = thread::current().id();
        
        // Create lock value with UUID and thread info
        let lock_value = format!("{}:{:?}:1", self.uuid, current_thread_id);
        
        // Try to acquire lock with expiration
        let result: bool = redis::cmd("SET")
            .arg(&lock_key)
            .arg(&lock_value)
            .arg("PX")
            .arg(self.lease_time_ms)
            .arg("NX")
            .query(&mut conn)?;
            
        if result {
            // Update local thread ownership tracking
            if let Ok(mut ownership_map) = self.thread_ownership.lock() {
                ownership_map.insert(self.lock_name.clone(), current_thread_id);
            }
        }
        
        Ok(result)
    }

    /// Release the lock (helper method for testing)
    pub fn unlock(&self) -> RedisResult<bool> {
        let mut conn = self.client.get_connection()?;
        let lock_key = format!("lock:{}", self.lock_name);
        let _current_thread_id = thread::current().id();
        
        // Lua script to safely release lock only if owned by current client
        let lua_script = r#"
            local lock_key = KEYS[1]
            local expected_prefix = ARGV[1]
            local current_value = redis.call('GET', lock_key)
            
            if current_value and string.sub(current_value, 1, string.len(expected_prefix)) == expected_prefix then
                redis.call('DEL', lock_key)
                return 1
            else
                return 0
            end
        "#;
        
        let result: i32 = redis::Script::new(lua_script)
            .key(&lock_key)
            .arg(&self.uuid)
            .invoke(&mut conn)?;
            
        if result == 1 {
            // Update local thread ownership tracking
            if let Ok(mut ownership_map) = self.thread_ownership.lock() {
                ownership_map.remove(&self.lock_name);
            }
        }
        
        Ok(result == 1)
    }

    /// Check if any thread holds the lock (not necessarily current thread)
    pub fn is_locked(&self) -> RedisResult<bool> {
        let mut conn = self.client.get_connection()?;
        let lock_key = format!("lock:{}", self.lock_name);
        
        let exists: bool = conn.exists(&lock_key)?;
        Ok(exists)
    }

    /// Get the remaining time to live for the lock in milliseconds
    pub fn get_remaining_time_to_live(&self) -> RedisResult<i64> {
        let mut conn = self.client.get_connection()?;
        let lock_key = format!("lock:{}", self.lock_name);
        
        let ttl: i64 = conn.pttl(&lock_key)?;
        Ok(ttl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use std::sync::mpsc;

    /// Test helper to create a lock repository similar to Java RedisLockRepository
    struct RedisLockRepository {
        redis_url: String,
    }

    impl RedisLockRepository {
        fn new(redis_url: &str) -> Self {
            Self {
                redis_url: redis_url.to_string(),
            }
        }

        fn try_lock(&self, lock_name: &str, lease_time_seconds: u64) -> Result<bool, Box<dyn std::error::Error>> {
            let lock = RedissonLock::new(&self.redis_url, lock_name, lease_time_seconds * 1000)?;
            Ok(lock.try_lock()?)
        }

        fn is_held_by_current_thread(&self, lock_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
            let lock = RedissonLock::new(&self.redis_url, lock_name, 30000)?;
            Ok(lock.is_held_by_current_thread()?)
        }

        fn unlock(&self, lock_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
            let lock = RedissonLock::new(&self.redis_url, lock_name, 30000)?;
            Ok(lock.unlock()?)
        }
    }

    #[test]
    fn test_is_held_by_current_thread() {
        // Note: This test requires a running Redis instance
        let lock = RedissonLock::new("redis://127.0.0.1/", "test_lock", 10000)
            .expect("Failed to create lock");

        // Initially, lock should not be held
        assert!(!lock.is_held_by_current_thread().unwrap());

        // Acquire lock
        assert!(lock.try_lock().unwrap());

        // Now it should be held by current thread
        assert!(lock.is_held_by_current_thread().unwrap());

        // Test from another thread
        let lock_clone = lock.clone();
        let handle = thread::spawn(move || {
            // From another thread, it should not be held by current thread
            !lock_clone.is_held_by_current_thread().unwrap()
        });
        
        assert!(handle.join().unwrap());

        // Release lock
        assert!(lock.unlock().unwrap());

        // Should no longer be held
        assert!(!lock.is_held_by_current_thread().unwrap());
    }

    #[test]
    fn test_lock_expiration() {
        let lock = RedissonLock::new("redis://127.0.0.1/", "test_expiration", 100)
            .expect("Failed to create lock");

        assert!(lock.try_lock().unwrap());
        assert!(lock.is_held_by_current_thread().unwrap());

        // Wait for expiration
        thread::sleep(Duration::from_millis(150));

        // Lock should be expired, but local tracking might still think we have it
        // The Redis verification should catch this
        let is_held = lock.is_held_by_current_thread().unwrap();
        assert!(!is_held, "Lock should have expired");
    }

    /// Test equivalent to: all_ok_with_embed_redis_when_use_lock_in_the_same_thread
    /// Note: Requires Redis running on localhost:6379
    #[test]
    fn test_all_ok_when_use_lock_in_the_same_thread() {
        let repository = RedisLockRepository::new("redis://127.0.0.1:6379/");

        // First lock attempt should succeed
        let first = repository.try_lock("MY_LOCK", 2).unwrap();
        assert!(first, "First lock attempt should succeed");

        // Second lock attempt should fail (lock already held)
        let second = repository.try_lock("MY_LOCK", 2).unwrap();
        assert!(!second, "Second lock attempt should fail");

        // Wait for lock to expire (2 seconds + buffer)
        thread::sleep(Duration::from_secs(3));

        // Third lock attempt should succeed (after expiration)
        let third = repository.try_lock("MY_LOCK", 2).unwrap();
        assert!(third, "Third lock attempt should succeed after expiration");

        // Clean up
        let _ = repository.unlock("MY_LOCK");
    }

    /// Test equivalent to: all_ok_with_embed_redis_when_use_lock_between_two_thread
    /// Note: Requires Redis running on localhost:6379
    #[test]
    fn test_all_ok_when_use_lock_between_two_threads() {
        let redis_url = "redis://127.0.0.1:6379/";
        let lock_name = "MY_LOCK_MULTI_THREAD";

        // Create channels for communication between threads
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();

        // First thread attempts to acquire lock
        let redis_url_clone1 = redis_url.to_string();
        let lock_name_clone1 = lock_name.to_string();
        let handle1 = thread::spawn(move || {
            let repository = RedisLockRepository::new(&redis_url_clone1);
            let result = repository.try_lock(&lock_name_clone1, 600).unwrap();
            tx1.send(result).unwrap();
            
            // Keep the thread alive to maintain the lock
            thread::sleep(Duration::from_millis(100));
            result
        });

        // Second thread attempts to acquire the same lock
        let redis_url_clone2 = redis_url.to_string();
        let lock_name_clone2 = lock_name.to_string();
        let handle2 = thread::spawn(move || {
            // Small delay to ensure first thread acquires lock first
            thread::sleep(Duration::from_millis(50));
            let repository = RedisLockRepository::new(&redis_url_clone2);
            let result = repository.try_lock(&lock_name_clone2, 600).unwrap();
            tx2.send(result).unwrap();
            result
        });

        // Verify results
        let first_result = rx1.recv().unwrap();
        let second_result = rx2.recv().unwrap();
        
        assert!(first_result, "First thread should acquire lock successfully");
        assert!(!second_result, "Second thread should fail to acquire lock");

        // Wait for threads to complete
        let _ = handle1.join();
        let _ = handle2.join();

        // Clean up
        let repository = RedisLockRepository::new(redis_url);
        let _ = repository.unlock(lock_name);
    }

    /// Test isHeldByCurrentThread behavior across threads
    #[test]
    fn test_is_held_by_current_thread_across_threads() {
        let redis_url = "redis://127.0.0.1:6379/";
        let lock_name = "THREAD_SPECIFIC_LOCK";

        let lock = RedissonLock::new(redis_url, lock_name, 30000).unwrap();
        
        // Initially no thread holds the lock
        assert!(!lock.is_held_by_current_thread().unwrap());

        // Acquire lock in main thread
        assert!(lock.try_lock().unwrap());
        assert!(lock.is_held_by_current_thread().unwrap());

        // Test from another thread - should return false
        let lock_clone = lock.clone();
        let handle = thread::spawn(move || {
            // This thread doesn't hold the lock, even though it exists in Redis
            lock_clone.is_held_by_current_thread().unwrap()
        });

        let other_thread_result = handle.join().unwrap();
        assert!(!other_thread_result, "Other thread should not see itself as holding the lock");

        // Main thread should still hold the lock
        assert!(lock.is_held_by_current_thread().unwrap());

        // Release lock
        assert!(lock.unlock().unwrap());
        assert!(!lock.is_held_by_current_thread().unwrap());
    }

    /// Test concurrent lock attempts with detailed verification
    #[test]
    fn test_concurrent_lock_attempts_with_verification() {
        let redis_url = "redis://127.0.0.1:6379/";
        let lock_name = "CONCURRENT_TEST_LOCK";
        let num_threads = 5;
        let lock_duration_ms = 1000;

        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let redis_url = redis_url.to_string();
                let lock_name = lock_name.to_string();
                
                thread::spawn(move || {
                    let lock = RedissonLock::new(&redis_url, &lock_name, lock_duration_ms).unwrap();
                    
                    let acquired = lock.try_lock().unwrap();
                    if acquired {
                        // Verify that this thread holds the lock
                        assert!(lock.is_held_by_current_thread().unwrap(), 
                               "Thread {} should hold the lock after acquiring it", thread_id);
                        
                        // Hold the lock briefly
                        thread::sleep(Duration::from_millis(100));
                        
                        // Release the lock
                        assert!(lock.unlock().unwrap(), 
                               "Thread {} should be able to unlock", thread_id);
                        
                        // Verify lock is no longer held
                        assert!(!lock.is_held_by_current_thread().unwrap(), 
                               "Thread {} should not hold lock after releasing", thread_id);
                    } else {
                        // If we didn't acquire the lock, we shouldn't hold it
                        assert!(!lock.is_held_by_current_thread().unwrap(), 
                               "Thread {} should not hold lock if acquisition failed", thread_id);
                    }
                    
                    (thread_id, acquired)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // Exactly one thread should have successfully acquired the lock
        let successful_acquisitions: Vec<_> = results.iter()
            .filter(|(_, acquired)| *acquired)
            .collect();
            
        assert_eq!(successful_acquisitions.len(), 1, 
                  "Exactly one thread should acquire the lock, got: {:?}", results);

        // Clean up
        let cleanup_lock = RedissonLock::new(redis_url, lock_name, 1000).unwrap();
        let _ = cleanup_lock.unlock();
    }

    /// Test lock expiration and reacquisition behavior
    #[test]
    fn test_lock_expiration_and_reacquisition() {
        let redis_url = "redis://127.0.0.1:6379/";
        let lock_name = "EXPIRATION_TEST_LOCK";
        let short_lease_ms = 500;

        let lock = RedissonLock::new(redis_url, lock_name, short_lease_ms).unwrap();

        // Acquire lock
        assert!(lock.try_lock().unwrap());
        assert!(lock.is_held_by_current_thread().unwrap());

        // Wait for expiration
        thread::sleep(Duration::from_millis(short_lease_ms + 100));

        // Lock should have expired
        assert!(!lock.is_held_by_current_thread().unwrap(), 
               "Lock should have expired");

        // Should be able to reacquire
        let new_lock = RedissonLock::new(redis_url, lock_name, 5000).unwrap();
        assert!(new_lock.try_lock().unwrap(), 
               "Should be able to reacquire expired lock");
        assert!(new_lock.is_held_by_current_thread().unwrap());

        // Clean up
        assert!(new_lock.unlock().unwrap());
    }
}

// Example usage
fn main() -> RedisResult<()> {
    use std::time::Duration;
    let lock = RedissonLock::new("redis://127.0.0.1/", "my_resource", 30000)?;

    println!("Is held by current thread: {}", lock.is_held_by_current_thread()?);
    
    if lock.try_lock()? {
        println!("Lock acquired!");
        println!("Is held by current thread: {}", lock.is_held_by_current_thread()?);
        println!("Lock TTL: {} ms", lock.get_remaining_time_to_live()?);
        
        // Do some work...
        thread::sleep(Duration::from_millis(1000));
        
        lock.unlock()?;
        println!("Lock released!");
    } else {
        println!("Failed to acquire lock");
    }
    
    println!("Is held by current thread: {}", lock.is_held_by_current_thread()?);
    
    Ok(())
}