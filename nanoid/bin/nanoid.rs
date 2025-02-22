
use nanoid::nanoid;

fn main() {
    let id = nanoid!();
    println!("{}",id);
    let id = nanoid!(10); 
    println!("{}",id);
    let alphabet: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f'
    ];

    let id = nanoid!(10, &alphabet); 
    println!("{}",id);


    {
        fn random (size: usize) -> Vec<u8> {
            let mut bytes: Vec<u8> = vec![0; size];
    
            for i in 0..size {
                bytes[i] = random_byte();
            }
    
            bytes
        }
   
       let id = nanoid!(10, &['a', 'b', 'c', 'd', 'e', 'f'], random);
       println!("{}",id);
    }
    let id = nanoid!(10, &nanoid::alphabet::SAFE, random);
    println!("{}",id);
}

fn random_byte () -> u8 { 3 }

fn random (size: usize) -> Vec<u8> {
    let mut result: Vec<u8> = vec![0; size];
    result[1] = 15;

    result
}