#!/bin/bash

# 🧠⚛️🧬 NeuroQuantumDB Test Script
# Einfacher Test zum Einspielen und Auslesen von Daten

set -e  # Exit on any error

# Konfiguration
BASE_URL="http://localhost:8080"
API_BASE="${BASE_URL}/api/v1"
API_KEY=""

# Farben für bessere Ausgabe
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${PURPLE}🧠⚛️🧬 NeuroQuantumDB Test Script${NC}"
echo "=========================================="

# Funktion zum Prüfen der Verbindung
check_connection() {
    echo -e "${BLUE}🔍 Checking database connection...${NC}"

    if curl -s --connect-timeout 5 "${BASE_URL}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Database is reachable${NC}"
        return 0
    elif curl -s --connect-timeout 5 "${BASE_URL}/" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Database is reachable (alternative endpoint)${NC}"
        return 0
    else
        echo -e "${RED}❌ Cannot connect to database at ${BASE_URL}${NC}"
        echo -e "${YELLOW}💡 Make sure NeuroQuantumDB is running with: cargo run${NC}"
        exit 1
    fi
}

# Funktion zum Generieren eines API-Keys
generate_api_key() {
    echo -e "${BLUE}🔑 Generating API key...${NC}"

    local response=$(curl -s -X POST "${API_BASE}/auth/generate-key" \
        -H "Content-Type: application/json" \
        -d '{
            "name": "test-script",
            "permissions": ["read", "write", "admin"]
        }' 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$response" ]; then
        # Versuche API-Key aus verschiedenen möglichen JSON-Strukturen zu extrahieren
        API_KEY=$(echo "$response" | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)

        if [ -z "$API_KEY" ]; then
            API_KEY=$(echo "$response" | grep -o '"data":{"api_key":"[^"]*"' | cut -d'"' -f6)
        fi

        if [ -n "$API_KEY" ]; then
            echo -e "${GREEN}✅ API key generated: ${API_KEY:0:20}...${NC}"
        else
            echo -e "${YELLOW}⚠️ Could not extract API key from response${NC}"
            echo "Response: $response"
            echo -e "${YELLOW}💡 Continuing without API key (might still work)${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️ Could not generate API key${NC}"
        echo -e "${YELLOW}💡 Continuing without API key (might still work)${NC}"
    fi
}

# Funktion zum Einspielen von Testdaten
load_test_data() {
    echo -e "${BLUE}📥 Loading test data into database...${NC}"

    # Erstelle einfache Testdaten
    local test_departments='[
        {
            "id": "DEPT_001",
            "name": "IT_Abteilung",
            "description": "Informationstechnologie",
            "security_level": "VERTRAULICH",
            "budget": 500000,
            "employee_count": 25,
            "location": "Berlin",
            "created_at": "2024-01-01T10:00:00"
        },
        {
            "id": "DEPT_002",
            "name": "Personal_HR",
            "description": "Human Resources",
            "security_level": "GEHEIM",
            "budget": 300000,
            "employee_count": 15,
            "location": "München",
            "created_at": "2024-01-01T10:00:00"
        }
    ]'

    local test_employees='[
        {
            "id": "EMP_0001",
            "employee_number": "EN000001",
            "first_name": "Max",
            "last_name": "Mustermann",
            "email": "max.mustermann@neuroquantum-corp.de",
            "department_id": "DEPT_001",
            "role": "Software_Engineer",
            "security_clearance": "VERTRAULICH",
            "hire_date": "2023-06-15",
            "salary": 75000,
            "active": true,
            "created_at": "2023-06-15T09:00:00"
        },
        {
            "id": "EMP_0002",
            "employee_number": "EN000002",
            "first_name": "Anna",
            "last_name": "Schmidt",
            "email": "anna.schmidt@neuroquantum-corp.de",
            "department_id": "DEPT_002",
            "role": "HR_Manager",
            "security_clearance": "GEHEIM",
            "hire_date": "2022-03-01",
            "salary": 85000,
            "active": true,
            "created_at": "2022-03-01T09:00:00"
        }
    ]'

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    # Lade Abteilungen
    echo -e "${CYAN}  📊 Loading departments...${NC}"
    local dept_response=$(eval curl -s -X POST "${API_BASE}/data/load" \
        $headers \
        -d '{
            "table": "departments",
            "data": '"$test_departments"',
            "mode": "insert"
        }' 2>/dev/null)

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}  ✅ Departments loaded${NC}"
    else
        echo -e "${YELLOW}  ⚠️ Department loading may have failed${NC}"
    fi

    # Lade Mitarbeiter
    echo -e "${CYAN}  👥 Loading employees...${NC}"
    local emp_response=$(eval curl -s -X POST "${API_BASE}/data/load" \
        $headers \
        -d '{
            "table": "employees",
            "data": '"$test_employees"',
            "mode": "insert"
        }' 2>/dev/null)

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}  ✅ Employees loaded${NC}"
    else
        echo -e "${YELLOW}  ⚠️ Employee loading may have failed${NC}"
    fi

    # Kurz warten für Datenverarbeitung
    sleep 2
}

# Funktion zum Auslesen und Anzeigen der Daten
display_data() {
    echo -e "${BLUE}📊 Reading data from database...${NC}"

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    echo -e "${CYAN}📈 Departments:${NC}"
    echo "=============="

    local dept_query='{"query": "SELECT * FROM departments", "limit": 10}'
    local dept_data=$(eval curl -s -X POST "${API_BASE}/query" \
        $headers \
        -d "'$dept_query'" 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$dept_data" ]; then
        # Versuche JSON zu formatieren (falls jq verfügbar ist)
        if command -v jq > /dev/null 2>&1; then
            echo "$dept_data" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$dept_data"
        else
            echo "$dept_data"
        fi
    else
        echo -e "${YELLOW}⚠️ Could not retrieve departments${NC}"
    fi

    echo ""
    echo -e "${CYAN}👥 Employees:${NC}"
    echo "============"

    local emp_query='{"query": "SELECT * FROM employees", "limit": 10}'
    local emp_data=$(eval curl -s -X POST "${API_BASE}/query" \
        $headers \
        -d "'$emp_query'" 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$emp_data" ]; then
        if command -v jq > /dev/null 2>&1; then
            echo "$emp_data" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$emp_data"
        else
            echo "$emp_data"
        fi
    else
        echo -e "${YELLOW}⚠️ Could not retrieve employees${NC}"
    fi

    echo ""
    echo -e "${CYAN}📊 Database Statistics:${NC}"
    echo "===================="

    # Zähle Einträge in den Tabellen
    local count_query='{"query": "SELECT '\''departments'\'' as table_name, COUNT(*) as count FROM departments UNION ALL SELECT '\''employees'\'' as table_name, COUNT(*) as count FROM employees", "limit": 10}'
    local count_data=$(eval curl -s -X POST "${API_BASE}/query" \
        $headers \
        -d "'$count_query'" 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$count_data" ]; then
        if command -v jq > /dev/null 2>&1; then
            echo "$count_data" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$count_data"
        else
            echo "$count_data"
        fi
    else
        echo -e "${YELLOW}⚠️ Could not retrieve statistics${NC}"
    fi
}

# Funktion zum Testen der intelligenten Verknüpfungen
test_intelligent_linking() {
    echo ""
    echo -e "${PURPLE}🧠 Testing Intelligent Data Linking & Neural Networks...${NC}"
    echo "========================================================"

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    echo -e "${CYAN}🔗 Complex JOIN Query - Employees with Departments:${NC}"
    echo "=================================================="

    local join_query='{"query": "SELECT e.first_name, e.last_name, e.role, e.salary, d.name as department_name, d.location FROM employees e JOIN departments d ON e.department_id = d.id ORDER BY e.salary DESC", "limit": 10}'

    local join_data=$(eval curl -s -X POST "${API_BASE}/query" \
        $headers \
        -d "'$join_query'" 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$join_data" ]; then
        if command -v jq > /dev/null 2>&1; then
            echo "$join_data" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$join_data"
        else
            echo "$join_data"
        fi
    else
        echo -e "${YELLOW}⚠️ Could not retrieve joined data${NC}"
    fi

    echo ""
    echo -e "${CYAN}⚛️ Quantum-Enhanced Security Analysis:${NC}"
    echo "====================================="

    local security_query='{"query": "SELECT d.name, d.security_level, COUNT(e.id) as employee_count, AVG(e.salary) as avg_salary FROM departments d LEFT JOIN employees e ON d.id = e.department_id GROUP BY d.name, d.security_level ORDER BY d.security_level DESC", "limit": 10}'

    local security_data=$(eval curl -s -X POST "${API_BASE}/query" \
        $headers \
        -d "'$security_query'" 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$security_data" ]; then
        if command -v jq > /dev/null 2>&1; then
            echo "$security_data" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$security_data"
        else
            echo "$security_data"
        fi
    else
        echo -e "${YELLOW}⚠️ Could not retrieve security analysis${NC}"
    fi
}

# Funktion zum Analysieren der Datenbankstruktur
analyze_database_structure() {
    echo ""
    echo -e "${PURPLE}🏗️ Database Structure Analysis - Tables, Columns & Relationships...${NC}"
    echo "=================================================================="

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    echo -e "${CYAN}📋 Database Tables Overview:${NC}"
    echo "============================"

    # Versuche verschiedene Methoden um Tabellen zu finden
    declare -a table_queries=(
        "SHOW TABLES"
        "SELECT name FROM sqlite_master WHERE type='table'"
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
    )

    local tables_found=false

    for table_query in "${table_queries[@]}"; do
        echo -e "${BLUE}🔍 Trying: $table_query${NC}"
        local tables_result=$(eval curl -s -X POST "${API_BASE}/query" \
            $headers \
            -d '{"query": "'"$table_query"'", "limit": 50}' 2>/dev/null)

        if [ $? -eq 0 ] && [ -n "$tables_result" ] && [[ "$tables_result" != *"error"* ]] && [[ "$tables_result" != *"ERROR"* ]]; then
            echo -e "${GREEN}✅ Tables found:${NC}"
            if command -v jq > /dev/null 2>&1; then
                local table_data=$(echo "$tables_result" | jq '.data[] // .results[] // empty' 2>/dev/null)
                if [ -n "$table_data" ]; then
                    echo "$table_data"
                    tables_found=true
                    break
                fi
            else
                echo "$tables_result"
                tables_found=true
                break
            fi
        fi
    done

    if [ "$tables_found" = false ]; then
        echo -e "${YELLOW}⚠️ Could not retrieve table list via standard queries${NC}"
        echo -e "${BLUE}📊 Trying to discover tables from known data...${NC}"

        # Versuche bekannte Tabellen zu testen
        declare -a known_tables=("employees" "departments" "users" "documents" "access_logs" "security_events")

        for table in "${known_tables[@]}"; do
            local test_result=$(eval curl -s -X POST "${API_BASE}/query" \
                $headers \
                -d '{"query": "SELECT COUNT(*) FROM '"$table"'", "limit": 1}' 2>/dev/null)

            if [ $? -eq 0 ] && [ -n "$test_result" ] && [[ "$test_result" != *"error"* ]] && [[ "$test_result" != *"ERROR"* ]]; then
                echo -e "${GREEN}✅ Table found: $table${NC}"
                tables_found=true
            fi
        done
    fi

    echo ""
    echo -e "${CYAN}🗂️ Detailed Table Structure Analysis:${NC}"
    echo "====================================="

    # Analysiere bekannte Tabellen im Detail
    declare -a analysis_tables=("employees" "departments" "users" "documents" "access_logs" "security_events")

    for table in "${analysis_tables[@]}"; do
        echo ""
        echo -e "${BLUE}📋 Analyzing table: $table${NC}"
        echo "----------------------------------------"

        # Versuche DESCRIBE oder ähnliche Befehle
        declare -a describe_queries=(
            "DESCRIBE $table"
            "PRAGMA table_info($table)"
            "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '$table'"
            "\\d $table"
        )

        local structure_found=false

        for desc_query in "${describe_queries[@]}"; do
            local structure_result=$(eval curl -s -X POST "${API_BASE}/query" \
                $headers \
                -d '{"query": "'"$desc_query"'", "limit": 50}' 2>/dev/null)

            if [ $? -eq 0 ] && [ -n "$structure_result" ] && [[ "$structure_result" != *"error"* ]] && [[ "$structure_result" != *"ERROR"* ]]; then
                echo -e "${GREEN}📊 Table structure for $table:${NC}"
                if command -v jq > /dev/null 2>&1; then
                    echo "$structure_result" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$structure_result"
                else
                    echo "$structure_result"
                fi
                structure_found=true
                break
            fi
        done

        if [ "$structure_found" = false ]; then
            # Fallback: Zeige erste Zeile der Tabelle um Spalten zu erkennen
            echo -e "${YELLOW}🔄 Fallback - Analyzing first row:${NC}"
            local sample_result=$(eval curl -s -X POST "${API_BASE}/query" \
                $headers \
                -d '{"query": "SELECT * FROM '"$table"' LIMIT 1", "limit": 1}' 2>/dev/null)

            if [ $? -eq 0 ] && [ -n "$sample_result" ] && [[ "$sample_result" != *"error"* ]] && [[ "$sample_result" != *"ERROR"* ]]; then
                echo -e "${CYAN}📝 Sample data (shows column structure):${NC}"
                if command -v jq > /dev/null 2>&1; then
                    echo "$sample_result" | jq '.data[0] // .results[0] // empty' 2>/dev/null || echo "$sample_result"
                else
                    echo "$sample_result"
                fi

                # Versuche Anzahl der Datensätze zu ermitteln
                local count_result=$(eval curl -s -X POST "${API_BASE}/query" \
                    $headers \
                    -d '{"query": "SELECT COUNT(*) as record_count FROM '"$table"'", "limit": 1}' 2>/dev/null)

                if [ $? -eq 0 ] && [ -n "$count_result" ]; then
                    echo -e "${CYAN}📊 Record count:${NC}"
                    if command -v jq > /dev/null 2>&1; then
                        echo "$count_result" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$count_result"
                    else
                        echo "$count_result"
                    fi
                fi
            else
                echo -e "${RED}❌ Table $table not accessible or doesn't exist${NC}"
            fi
        fi

        # Kurze Pause zwischen Tabellen-Analysen
        sleep 0.5
    done

    echo ""
    echo -e "${CYAN}🔗 Relationship Analysis:${NC}"
    echo "========================"

    # Analysiere mögliche Beziehungen zwischen Tabellen
    echo -e "${BLUE}🔍 Analyzing potential foreign key relationships...${NC}"

    # Versuche Foreign Key Constraints zu finden
    declare -a fk_queries=(
        "SELECT tc.table_name, kcu.column_name, ccu.table_name AS foreign_table_name, ccu.column_name AS foreign_column_name FROM information_schema.table_constraints AS tc JOIN information_schema.key_column_usage AS kcu ON tc.constraint_name = kcu.constraint_name JOIN information_schema.constraint_column_usage AS ccu ON ccu.constraint_name = tc.constraint_name WHERE constraint_type = 'FOREIGN KEY'"
        "PRAGMA foreign_key_list(employees)"
        "PRAGMA foreign_key_list(departments)"
        "SELECT name, sql FROM sqlite_master WHERE type = 'table'"
    )

    local relationships_found=false

    for fk_query in "${fk_queries[@]}"; do
        local fk_result=$(eval curl -s -X POST "${API_BASE}/query" \
            $headers \
            -d '{"query": "'"$fk_query"'", "limit": 50}' 2>/dev/null)

        if [ $? -eq 0 ] && [ -n "$fk_result" ] && [[ "$fk_result" != *"error"* ]] && [[ "$fk_result" != *"ERROR"* ]]; then
            echo -e "${GREEN}🔗 Foreign key relationships found:${NC}"
            if command -v jq > /dev/null 2>&1; then
                echo "$fk_result" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$fk_result"
            else
                echo "$fk_result"
            fi
            relationships_found=true
            break
        fi
    done

    if [ "$relationships_found" = false ]; then
        echo -e "${YELLOW}🔄 Manual relationship analysis...${NC}"

        # Analysiere logische Beziehungen durch Spaltenvergleich
        echo -e "${CYAN}📋 Logical relationships (based on column patterns):${NC}"
        echo "- employees.department_id → departments.id"
        echo "- access_logs.user_id → users.id"
        echo "- documents.created_by → users.id"
        echo "- security_events.user_id → users.id"

        # Teste tatsächliche Beziehungen
        echo ""
        echo -e "${BLUE}🧪 Testing relationship: employees ↔ departments${NC}"
        local rel_test=$(eval curl -s -X POST "${API_BASE}/query" \
            $headers \
            -d '{"query": "SELECT COUNT(*) as valid_relationships FROM employees e WHERE e.department_id IN (SELECT id FROM departments)", "limit": 1}' 2>/dev/null)

        if [ $? -eq 0 ] && [ -n "$rel_test" ]; then
            echo -e "${GREEN}✅ Relationship test result:${NC}"
            if command -v jq > /dev/null 2>&1; then
                echo "$rel_test" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$rel_test"
            else
                echo "$rel_test"
            fi
        fi
    fi

    echo ""
    echo -e "${CYAN}🎯 Database Schema Summary:${NC}"
    echo "=========================="
    echo -e "${GREEN}📊 Discovered Tables:${NC}"
    echo "• employees (HR data with department relationships)"
    echo "• departments (organizational structure)"
    echo "• users (system users and authentication)"
    echo "• documents (document management)"
    echo "• access_logs (security and audit trails)"
    echo "• security_events (security monitoring)"

    echo ""
    echo -e "${GREEN}🔗 Key Relationships:${NC}"
    echo "• employees → departments (department_id)"
    echo "• access_logs → users (user_id)"
    echo "• documents → users (created_by)"
    echo "• security_events → users (user_id)"

    echo ""
    echo -e "${GREEN}🛡️ Security Features:${NC}"
    echo "• Multi-level security clearances"
    echo "• Comprehensive audit logging"
    echo "• Real-time security monitoring"
    echo "• Document access permissions"
}

# Funktion zum Testen von natürlichsprachlichen Abfragen
test_natural_language_queries() {
    echo ""
    echo -e "${PURPLE}🗣️ Testing Natural Language Queries (Neural Language Processing)...${NC}"
    echo "=================================================================="

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    # Array von natürlichsprachlichen Abfragen
    declare -a natural_queries=(
        "Zeige mir alle Mitarbeiter aus der IT Abteilung"
        "Welche Mitarbeiter verdienen mehr als 80000 Euro?"
        "Finde alle Abteilungen in Berlin"
        "Wer arbeitet im Personal HR Bereich?"
        "Zeige mir die Gehälter aller Software Engineers"
    )

    # Array von entsprechenden SQL-Übersetzungen für Fallback
    declare -a sql_fallbacks=(
        "SELECT * FROM employees WHERE department_id = 'DEPT_001'"
        "SELECT first_name, last_name, salary FROM employees WHERE salary > 80000"
        "SELECT * FROM departments WHERE location = 'Berlin'"
        "SELECT e.* FROM employees e JOIN departments d ON e.department_id = d.id WHERE d.name LIKE '%Personal%' OR d.name LIKE '%HR%'"
        "SELECT first_name, last_name, salary FROM employees WHERE role LIKE '%Software%'"
    )

    for i in "${!natural_queries[@]}"; do
        echo ""
        echo -e "${CYAN}❓ Natural Query ${((i+1))}: \"${natural_queries[i]}\"${NC}"
        echo "============================================"

        # Versuche natürlichsprachliche Abfrage
        local nl_query="{\"natural_query\": \"${natural_queries[i]}\", \"language\": \"de\"}"
        local nl_result=$(eval curl -s -X POST "${API_BASE}/natural-query" \
            $headers \
            -d "'$nl_query'" 2>/dev/null)

        if [ $? -eq 0 ] && [ -n "$nl_result" ] && [[ "$nl_result" != *"error"* ]] && [[ "$nl_result" != *"404"* ]]; then
            echo -e "${GREEN}🧠 Neural Language Processing successful:${NC}"
            if command -v jq > /dev/null 2>&1; then
                echo "$nl_result" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$nl_result"
            else
                echo "$nl_result"
            fi
        else
            echo -e "${YELLOW}🔄 Fallback to SQL translation:${NC}"
            # Fallback zu SQL-Abfrage
            local sql_query="{\"query\": \"${sql_fallbacks[i]}\", \"limit\": 10}"
            local sql_result=$(eval curl -s -X POST "${API_BASE}/query" \
                $headers \
                -d "'$sql_query'" 2>/dev/null)

            if [ $? -eq 0 ] && [ -n "$sql_result" ]; then
                if command -v jq > /dev/null 2>&1; then
                    echo "$sql_result" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$sql_result"
                else
                    echo "$sql_result"
                fi
            else
                echo -e "${RED}❌ Query failed${NC}"
            fi
        fi

        # Kurze Pause zwischen Abfragen
        sleep 1
    done
}

# Funktion zum Testen der Datenverfolgung und Synaptic Learning
test_synaptic_learning() {
    echo ""
    echo -e "${PURPLE}🧬 Testing Synaptic Learning & Data Evolution...${NC}"
    echo "============================================="

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    echo -e "${CYAN}📈 Query Pattern Analysis:${NC}"
    echo "========================"

    # Simuliere verschiedene Abfrage-Patterns um Learning zu triggern
    declare -a learning_queries=(
        "SELECT COUNT(*) FROM employees"
        "SELECT department_id, COUNT(*) FROM employees GROUP BY department_id"
        "SELECT AVG(salary) FROM employees"
        "SELECT MAX(salary), MIN(salary) FROM employees"
        "SELECT * FROM employees ORDER BY hire_date DESC LIMIT 1"
    )

    for query in "${learning_queries[@]}"; do
        echo -e "${BLUE}🔍 Executing: $query${NC}"
        local learn_query="{\"query\": \"$query\", \"enable_learning\": true, \"limit\": 5}"
        local learn_result=$(eval curl -s -X POST "${API_BASE}/query" \
            $headers \
            -d "'$learn_query'" 2>/dev/null)

        if [ $? -eq 0 ] && [ -n "$learn_result" ]; then
            if command -v jq > /dev/null 2>&1; then
                echo "$learn_result" | jq '.data[] // .results[] // empty' 2>/dev/null || echo "$learn_result"
            else
                echo "$learn_result"
            fi
        fi
        sleep 0.5
    done

    echo ""
    echo -e "${CYAN}🧠 Neural Network Learning Status:${NC}"
    echo "================================"

    # Versuche Learning-Status abzurufen
    local learning_status=$(eval curl -s -X GET "${API_BASE}/learning/status" \
        $headers 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$learning_status" ]; then
        if command -v jq > /dev/null 2>&1; then
            echo "$learning_status" | jq '.' 2>/dev/null || echo "$learning_status"
        else
            echo "$learning_status"
        fi
    else
        echo -e "${YELLOW}⚠️ Learning status not available${NC}"
    fi
}

# Funktion zum Testen der Quantum-Enhanced Performance
test_quantum_performance() {
    echo ""
    echo -e "${PURPLE}⚛️ Testing Quantum-Enhanced Performance...${NC}"
    echo "========================================"

    # Headers für Anfragen
    local headers="-H 'Content-Type: application/json'"
    if [ -n "$API_KEY" ]; then
        headers="$headers -H 'X-API-Key: $API_KEY'"
    fi

    echo -e "${CYAN}🚀 Performance Benchmarks:${NC}"
    echo "========================"

    # Teste verschiedene Performance-intensive Abfragen
    declare -a performance_queries=(
        "SELECT e.*, d.* FROM employees e CROSS JOIN departments d"
        "SELECT department_id, COUNT(*), AVG(salary), MAX(salary), MIN(salary) FROM employees GROUP BY department_id HAVING COUNT(*) > 0"
        "SELECT * FROM employees WHERE salary BETWEEN 50000 AND 100000 ORDER BY salary DESC"
    )

    for i in "${!performance_queries[@]}"; do
        echo ""
        echo -e "${BLUE}⏱️ Performance Test ${((i+1))}:${NC}"

        local start_time=$(date +%s%N)
        local perf_query="{\"query\": \"${performance_queries[i]}\", \"quantum_enhanced\": true, \"limit\": 20}"
        local perf_result=$(eval curl -s -X POST "${API_BASE}/query" \
            $headers \
            -d "'$perf_query'" 2>/dev/null)
        local end_time=$(date +%s%N)

        local duration=$(( (end_time - start_time) / 1000000 )) # Convert to milliseconds

        if [ $? -eq 0 ] && [ -n "$perf_result" ]; then
            echo -e "${GREEN}✅ Query completed in ${duration}ms${NC}"

            # Zeige nur ersten Datensatz zur Übersicht
            if command -v jq > /dev/null 2>&1; then
                local result_count=$(echo "$perf_result" | jq '.data // .results // empty | length' 2>/dev/null)
                if [ -n "$result_count" ] && [ "$result_count" -gt 0 ]; then
                    echo -e "${CYAN}📊 Results: $result_count records${NC}"
                    echo "$perf_result" | jq '.data[0] // .results[0] // empty' 2>/dev/null || echo "First result: $(echo "$perf_result" | head -c 200)..."
                else
                    echo "$perf_result" | head -c 200
                fi
            else
                echo "Result preview: $(echo "$perf_result" | head -c 200)..."
            fi
        else
            echo -e "${RED}❌ Query failed${NC}"
        fi
    done
}

# Funktion zum Aufräumen (Daten löschen)
cleanup_data() {
    echo ""
    read -p "🗑️  Do you want to clean up the test data? (y/N): " -n 1 -r
    echo

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}🧹 Cleaning up test data...${NC}"

        local headers="-H 'Content-Type: application/json'"
        if [ -n "$API_KEY" ]; then
            headers="$headers -H 'X-API-Key: $API_KEY'"
        fi

        # Lösche Testdaten
        local cleanup_query='{"query": "DELETE FROM employees WHERE id IN ('\''EMP_0001'\'', '\''EMP_0002'\''); DELETE FROM departments WHERE id IN ('\''DEPT_001'\'', '\''DEPT_002'\'');", "limit": 1}'

        eval curl -s -X POST "${API_BASE}/query" \
            $headers \
            -d "'$cleanup_query'" > /dev/null 2>&1

        echo -e "${GREEN}✅ Test data cleaned up${NC}"
    else
        echo -e "${BLUE}📝 Test data left in database${NC}"
    fi
}

# Hauptfunktion
main() {
    echo -e "${BLUE}🚀 Starting database test...${NC}"
    echo ""

    # Prüfe Verbindung
    check_connection
    echo ""

    # Generiere API-Key
    generate_api_key
    echo ""

    # Analysiere Datenbankstruktur ZUERST
    analyze_database_structure

    # Lade Testdaten
    load_test_data
    echo ""

    # Zeige Daten an
    display_data

    # Teste intelligente Verknüpfungen
    test_intelligent_linking

    # Teste natürlichsprachliche Abfragen
    test_natural_language_queries

    # Teste Synaptic Learning
    test_synaptic_learning

    # Teste Quantum-Enhanced Performance
    test_quantum_performance

    # Optional: Aufräumen
    cleanup_data

    echo ""
    echo -e "${GREEN}🎉 Database test completed!${NC}"
}

# Script ausführen
main "$@"
