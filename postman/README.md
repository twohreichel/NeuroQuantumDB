# NeuroQuantumDB Postman Collection

This Postman Collection allows you to test all API endpoints of NeuroQuantumDB locally.

## 📦 Contents

- `NeuroQuantumDB.postman_collection.json` - Complete API collection with all endpoints
- `NeuroQuantumDB.postman_environment.json` - Environment configuration for local testing
- `README.md` - This guide

## 🚀 Quickstart

### 1. Import Postman Collection

1. Open Postman
2. Click on **Import** (top left)
3. Select **File** and import:
   - `NeuroQuantumDB.postman_collection.json`
   - `NeuroQuantumDB.postman_environment.json`
4. The collection appears under "Collections" and the environment under "Environments"

### 2. Activate Environment

1. Click on the Environment dropdown in the top right
2. Select **"NeuroQuantumDB Local"**
3. The environment is now active and shows `http://localhost:8080` as base URL

### 3. Start API Server

Make sure the NeuroQuantumDB API server is running:

```bash
cd /Users/andreasreichel/workspace/NeuroQuantumDB
cargo run --bin neuroquantum-api
```

The server starts by default on `http://localhost:8080`.

### 4. Testing the API

#### Health Check (without authentication)
1. Open the collection **NeuroQuantumDB API**
2. Navigate to **Health & Status** → **Health Check**
3. Click **Send**
4. You should receive a successful response with status "healthy"

#### Login & Token Authentication
1. Navigate to **Authentication** → **Login**
2. Click **Send**
3. **The access token is automatically extracted and saved!**
4. All subsequent requests use this token automatically

## 🔑 Automatic Token Management

The collection contains post-response scripts that automatically:

- ✅ Extract **Access Token** from login response
- ✅ Save **Refresh Token**
- ✅ Save **User ID**
- ✅ Save **API Keys** after generation
- ✅ Provide **Network IDs** and other IDs for subsequent requests

You don't need to copy or paste anything manually!

## 📋 API Endpoints Overview

### Health & Status
- **Health Check** - Checks server status (no auth required)

### Authentication
- **Login** - Authentication with username/password → generates access token
- **Refresh Token** - Renews the access token
- **Generate API Key** - Creates a new API key (admin permission required)
- **Revoke API Key** - Revokes an API key (admin permission required)

### CRUD Operations
- **Execute SQL Query** - Executes arbitrary SQL queries
- **Create Table** - Creates a new table with schema
- **Insert Data** - Inserts data in batch
- **Query Data** - Queries data with filters
- **Update Data** - Updates records
- **Delete Data** - Deletes records (with soft-delete and cascade option)

### Neural Networks
- **Train Neural Network** - Starts training a neural network
- **Get Training Status** - Retrieves training status

### Quantum Search
- **Quantum Search** - Performs quantum-inspired search using Grover's algorithm

### DNA Compression
- **Compress DNA** - Compresses DNA sequences with advanced algorithms

### Biometric Authentication
- **EEG Enroll User** - Registers user with EEG biometric signature
- **EEG Authenticate** - Authenticates with EEG data
- **EEG Update Signature** - Updates EEG signature
- **EEG List Users** - Lists all registered EEG users

### Monitoring
- **Get Metrics** - Prometheus-compatible metrics
- **Get Performance Stats** - Detailed performance statistics

## 🔐 Authentication

The collection supports two authentication methods:

### 1. JWT Bearer Token (recommended for testing)
- Automatically used after login
- Automatically sent with all protected endpoints
- Token expires after 24 hours (can be renewed with refresh token)

### 2. API Key Authentication
- Can be created via **Generate API Key**
- Requires admin permission
- Suitable for long-term access

## 📝 Example Workflow

### Complete Test Run:

1. **Health Check** - Check server status
2. **Login** - Authenticate (token is automatically saved)
3. **Generate API Key** - Create an admin API key (optional)
4. **Create Table** - Create a "users" table
5. **Insert Data** - Insert test data
6. **Query Data** - Query the data
7. **Update Data** - Update a record
8. **Train Neural Network** - Start neural network training
9. **Get Training Status** - Check training progress
10. **Quantum Search** - Perform a quantum search
11. **Compress DNA** - Compress DNA sequences
12. **EEG Enroll User** - Register a user with EEG
13. **EEG Authenticate** - Authenticate with EEG data
14. **Get Performance Stats** - Retrieve performance metrics

## 🧪 Tests

Each request contains automatic tests:

```javascript
pm.test("Status code is 200", function () {
    pm.response.to.have.status(200);
});

pm.test("Response has success status", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData.success).to.be.true;
});
```

Tests are automatically executed and show green checkmarks on success.

## 🔧 Environment Variables

The environment contains the following variables:

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `base_url` | API Base URL | `http://localhost:8080` |
| `access_token` | JWT Access Token | Automatically set |
| `refresh_token` | JWT Refresh Token | Automatically set |
| `api_key` | Generated API Key | Automatically set |
| `user_id` | User ID | Automatically set |
| `table_name` | Default table name | `users` |
| `network_id` | Neural Network ID | Automatically set |
| `eeg_user_id` | EEG User ID | `john_doe_123` |

You can manually adjust these variables if desired.

## 🌐 Other Environments

For other environments (e.g., Production, Staging):

1. Duplicate the environment
2. Change the `base_url` accordingly:
   - Production: `https://api.neuroquantum.com`
   - Staging: `https://staging-api.neuroquantum.com`

## 🐛 Troubleshooting

### Problem: "Could not send request" / Connection refused
**Solution:** Make sure the API server is running:
```bash
cargo run --bin neuroquantum-api
```

### Problem: 401 Unauthorized
**Solution:** 
1. First execute the **Login** request
2. The token is automatically saved
3. Or use **Refresh Token** if the token has expired

### Problem: 403 Forbidden
**Solution:** The endpoint requires special permissions (e.g., admin)
1. Log in with an admin account
2. Or generate an API key with the required permissions

### Problem: Environment variables are not being set
**Solution:**
1. Check if the correct environment is selected (top right)
2. Look at the **Test** scripts of the requests (under "Tests" tab)
3. Open the console (View → Show Postman Console) for debug logs

## 📚 Additional Resources

- [API Documentation](http://localhost:8080/api-docs/) - Swagger UI (when server is running)
- [Project README](../README.md) - Main documentation
- [Development Guide](../docs/development/) - Developer documentation

## 🎯 Tips

1. **Collection Runner**: Run the entire collection automatically
   - Right-click on collection → "Run collection"
   - Useful for regression tests

2. **Code Generation**: Generate code for different languages
   - Click on a request → "Code" (right side)
   - Supports curl, Python, JavaScript, Go, etc.

3. **Environment Switcher**: Quickly switch between environments
   - Create different environments for Dev, Staging, Production

4. **Pre-request Scripts**: Add your own scripts
   - Generate dynamic data
   - Execute setup code

## 📞 Support

For questions or issues:
- Open an issue in the GitHub repository
- Consult the API documentation at `/api-docs/`

---

**Happy Testing! 🚀**

