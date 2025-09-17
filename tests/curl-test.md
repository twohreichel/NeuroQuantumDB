````powershell
sleep 3 && curl -s -X POST "http://localhost:8080/api/v1/query" \
-H "Content-Type: application/json" \
-d '{"query": "NEUROMATCH employees WHERE salary pattern similar weight 0.8", "quantum_enhanced": true, "enable_learning": true, "limit": 10}' | head -c 500
````

````powershell
sleep 3 && curl -s -X POST "http://localhost:8080/api/v1/query" \
-H "Content-Type: application/json" \
-d '{"query": "NQUANTUM_SEARCH departments WITH AMPLITUDE_AMPLIFICATION", "quantum_enhanced": true, "enable_learning": true, "limit": 10}' | head -c 500
````

````powershell
sleep 3 && curl -s -X POST "http://localhost:8080/api/v1/query" \
-H "Content-Type: application/json" \
-d '{"query": "show users with neural matching for high engagement patterns", "quantum_enhanced": true, "enable_learning": true, "limit": 10}' | head -c 500
````

````powershell
sleep 3 && curl -s -X POST "http://localhost:8080/api/v1/query" \
-H "Content-Type: application/json" \
-d '{"query": "quantum search employees where role patterns match software", "quantum_enhanced": true, "enable_learning": true, "limit": 10}' | head -c 500
````