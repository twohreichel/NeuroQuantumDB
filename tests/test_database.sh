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

    # Lade Testdaten
    load_test_data
    echo ""

    # Zeige Daten an
    display_data

    # Optional: Aufräumen
    cleanup_data

    echo ""
    echo -e "${GREEN}🎉 Database test completed!${NC}"
}

# Script ausführen
main "$@"
