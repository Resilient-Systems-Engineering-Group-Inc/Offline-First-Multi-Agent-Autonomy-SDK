# Auth Module

Authentication and authorization for the Offline-First Multi-Agent Autonomy SDK.

## Features

- JWT token generation and validation
- Password hashing with bcrypt
- API token management
- Role-Based Access Control (RBAC)
- Warp middleware for authentication

## Usage

### Authentication

```rust
use auth::{AuthConfig, JwtHandler, PasswordHasher, LoginRequest};

// Create auth config
let config = AuthConfig::default();
let jwt_handler = JwtHandler::new(&config.jwt_secret);

// Hash password
let password = "secure_password";
let hash = PasswordHasher::hash(password).unwrap();

// Verify password
let is_valid = PasswordHasher::verify(password, &hash).unwrap();

// Generate JWT token
let token = jwt_handler.generate_access_token(
    "user-123",
    "username",
    vec!["user".to_string(), "admin".to_string()]
).unwrap();

// Validate token
let user_id = jwt_handler.validate(&token).unwrap();
```

### RBAC

```rust
use auth::{RbacManager, ResourceType, Action, predefined_roles};

// Create RBAC manager
let rbac = RbacManager::new();

// Check permissions
let has_permission = rbac
    .check_permission(
        &["admin".to_string()],
        &ResourceType::Task,
        &Action::Delete
    )
    .await;

// Predefined roles
let admin_role = predefined_roles::admin();
let operator_role = predefined_roles::operator();
let viewer_role = predefined_roles::viewer();
```

### Warp Middleware

```rust
use auth::middleware::{auth_required, require_permission};
use auth::{ResourceType, Action};

// Require authentication
let protected_route = warp::path!("api" / "tasks")
    .and(auth_required(jwt_handler))
    .and_then(handle_tasks);

// Require specific permission
let admin_route = warp::path!("api" / "admin")
    .and(require_permission(
        rbac,
        ResourceType::System,
        Action::Admin
    ))
    .and_then(handle_admin);
```

## API Endpoints

### Authentication

```
POST /api/auth/login
POST /api/auth/register
POST /api/auth/refresh
POST /api/auth/logout
POST /api/auth/password/reset
```

### User Management

```
GET    /api/users
GET    /api/users/:id
PUT    /api/users/:id
DELETE /api/users/:id
```

### API Tokens

```
GET    /api/tokens
POST   /api/tokens
DELETE /api/tokens/:id
```

## Roles

### Admin
- Full access to all resources
- Can manage users and system settings
- Can view audit logs

### Operator
- Create, update, execute tasks and workflows
- Can pause, resume, cancel workflows
- Read-only access to agents

### Viewer
- Read-only access to all resources
- Can view tasks, workflows, agents
- Can view audit logs

### Agent
- Read and execute assigned tasks
- Update task status
- Read-only access to workflows

## Security

- Passwords hashed with bcrypt (cost factor 12)
- JWT tokens signed with HS256
- Token expiry: 24 hours (access), 30 days (refresh)
- API tokens with configurable expiry
- Rate limiting on authentication endpoints
- Audit logging for all auth events

## Configuration

Environment variables:
```bash
AUTH_JWT_SECRET=your-secret-key
AUTH_JWT_EXPIRY_HOURS=24
AUTH_REFRESH_TOKEN_EXPIRY_DAYS=30
AUTH_API_TOKEN_EXPIRY_DAYS=90
AUTH_RATE_LIMIT=100  # requests per minute
```

## Testing

```bash
cargo test -p auth
```

## License

MIT OR Apache-2.0
