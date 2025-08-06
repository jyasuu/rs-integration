
## Example GraphQL Queries

### Create User
```graphql
mutation {
  createUser(input: {
    email: "john@example.com"
    name: "John Doe"
  }) {
    id
    email
    name
    isActive
    createdAt
    updatedAt
  }
}
```

### Get User by ID
```graphql
query {
  user(id: "550e8400-e29b-41d4-a716-446655440000") {
    id
    email
    name
    isActive
    createdAt
    updatedAt
  }
}
```

### Get Users with Filtering and Pagination
```graphql
query {
  users(
    filter: {
      name: "John"
      isActive: true
    }
    pagination: {
      offset: 0
      limit: 10
    }
  ) {
    id
    email
    name
    isActive
    createdAt
    updatedAt
  }
}
```

### Update User
```graphql
mutation {
  updateUser(
    id: "550e8400-e29b-41d4-a716-446655440000"
    input: {
      name: "John Smith"
      isActive: false
    }
  ) {
    id
    email
    name
    isActive
    updatedAt
  }
}
```

### Delete User
```graphql
mutation {
  deleteUser(id: "550e8400-e29b-41d4-a716-446655440000")
}
```

## Running the Application

1. **Setup Database:**
   ```bash
   createdb rust_graphql_db
   ```

2. **Install and Run:**
   ```bash
   cargo run
   ```

3. **Access GraphQL Playground:**
   Open http://localhost:8000 in your browser

## Key Best Practices Implemented

1. **Layered Architecture**: Clear separation between models, services, and schema layers
2. **Error Handling**: Custom error types with proper GraphQL error extensions
3. **Validation**: Input validation using the validator crate
4. **Type Safety**: Strong typing throughout with proper error handling
5. **Database Query Builder**: Using Sea Query for type-safe SQL generation
6. **Pagination**: Built-in pagination support for list queries
7. **Filtering**: Flexible filtering system for queries
8. **Migrations**: Proper database migration support
9. **Configuration**: Environment-based configuration
10. **Logging**: Structured logging with tracing
11. **CORS**: Proper CORS configuration for web clients
12. **Security**: UUID-based primary keys and input sanitization
Improve
Explain
