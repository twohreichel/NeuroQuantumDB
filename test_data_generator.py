#!/usr/bin/env python3
"""
🧠⚛️🧬 NeuroQuantumDB Enterprise Data Generator & Test Suite
===========================================================

Generiert 500.000 realistische Unternehmensdaten für ein Szenario mit:
- 800 Mitarbeiter in 25 Abteilungen
- Sicherheitskritische Dokumente mit Zugriffsrechten
- Komplexe Verknüpfungen und Hierarchien
- Training der neuromorphen und Quantum-Systeme

Autor: NeuroQuantumDB Team
Version: 1.0.0
"""

import random
import string
import json
import time
import asyncio
import requests
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional
import uuid
import hashlib
from faker import Faker
import argparse
import logging

# Konfiguration
BASE_URL = "http://localhost:8080"
API_BASE = f"{BASE_URL}/api/v1"

# Data Generation Configuration
TOTAL_EMPLOYEES = 800
TOTAL_DEPARTMENTS = 25
TOTAL_DOCUMENTS = 150000
TOTAL_ACCESS_LOGS = 200000
TOTAL_PROJECTS = 5000
TOTAL_SECURITY_EVENTS = 15000
BATCH_SIZE = 1000

# Logging Setup
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Initialize Faker
fake = Faker(['de_DE', 'en_US'])

class EnterpriseDataGenerator:
    """🏢 Generator für realistische Unternehmensdaten"""

    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url
        self.api_base = f"{base_url}/api/v1"
        self.api_key = None
        self.session = requests.Session()

        # Data containers
        self.departments = []
        self.employees = []
        self.documents = []
        self.projects = []
        self.access_logs = []
        self.security_events = []

        # Security Classifications
        self.security_levels = [
            "ÖFFENTLICH", "INTERN", "VERTRAULICH", "GEHEIM", "STRENG_GEHEIM"
        ]

        # Document Types
        self.document_types = [
            "Verträge", "Finanzberichte", "Personalakten", "Technische_Spezifikationen",
            "Strategische_Pläne", "Compliance_Dokumente", "Forschung_Entwicklung",
            "Kundenakten", "Lieferantenverträge", "Sicherheitsrichtlinien"
        ]

        # Department Names (German enterprise structure)
        self.department_names = [
            "Geschäftsführung", "Finanzen_Controlling", "Personal_HR", "IT_Digitalisierung",
            "Forschung_Entwicklung", "Produktion", "Qualitätssicherung", "Vertrieb",
            "Marketing", "Einkauf", "Logistik", "Kundendienst", "Recht_Compliance",
            "Sicherheit", "Facility_Management", "Business_Intelligence", "Projektmanagement",
            "Risikomanagement", "Interne_Revision", "Kommunikation_PR", "Umwelt_Nachhaltigkeit",
            "Innovation_Lab", "Datenanalyse", "Cybersecurity", "Change_Management"
        ]

    def generate_departments(self) -> List[Dict]:
        """🏢 Generiere realistische Abteilungen"""
        logger.info(f"Generating {TOTAL_DEPARTMENTS} departments...")

        departments = []

        for i in range(TOTAL_DEPARTMENTS):
            dept_id = f"DEPT_{i+1:03d}"
            dept_name = self.department_names[i]

            # Bestimme Sicherheitslevel der Abteilung
            if dept_name in ["Geschäftsführung", "Sicherheit", "Cybersecurity", "Recht_Compliance"]:
                dept_security_level = "STRENG_GEHEIM"
            elif dept_name in ["Finanzen_Controlling", "Personal_HR", "Interne_Revision"]:
                dept_security_level = "GEHEIM"
            elif dept_name in ["Forschung_Entwicklung", "IT_Digitalisierung", "Risikomanagement"]:
                dept_security_level = "VERTRAULICH"
            else:
                dept_security_level = random.choice(["INTERN", "VERTRAULICH"])

            department = {
                "id": dept_id,
                "name": dept_name,
                "description": f"Abteilung für {dept_name.replace('_', ' ')}",
                "security_level": dept_security_level,
                "budget": random.randint(100000, 5000000),
                "employee_count": random.randint(15, 50),
                "location": random.choice(["Berlin", "München", "Hamburg", "Frankfurt", "Köln"]),
                "manager_id": None,  # Wird später gesetzt
                "parent_department": "DEPT_001" if i > 0 and random.random() < 0.3 else None,
                "cost_center": f"CC_{i+1:04d}",
                "created_at": fake.date_time_between(start_date='-2y', end_date='now').isoformat()
            }

            departments.append(department)

        self.departments = departments
        return departments

    def generate_employees(self) -> List[Dict]:
        """👥 Generiere realistische Mitarbeiter"""
        logger.info(f"Generating {TOTAL_EMPLOYEES} employees...")

        employees = []

        # Setze Manager für Abteilungen
        for i, dept in enumerate(self.departments):
            self.departments[i]["manager_id"] = f"EMP_{i+1:04d}"

        for i in range(TOTAL_EMPLOYEES):
            emp_id = f"EMP_{i+1:04d}"

            # Zufällige Abteilung
            department = random.choice(self.departments)

            # Bestimme Rolle basierend auf Position
            if i < TOTAL_DEPARTMENTS:  # Erste X sind Manager
                role = "Abteilungsleiter"
                security_clearance = department["security_level"]
            elif i < TOTAL_DEPARTMENTS * 3:  # Nächste sind Senior
                role = random.choice(["Senior_Spezialist", "Teamleiter", "Projektleiter"])
                security_clearance = department["security_level"]
            else:
                role = random.choice(["Spezialist", "Sachbearbeiter", "Analyst", "Coordinator"])
                # Junior Mitarbeiter haben eventuell niedrigere Berechtigung
                if random.random() < 0.3:
                    clearance_levels = ["ÖFFENTLICH", "INTERN", "VERTRAULICH", "GEHEIM", "STRENG_GEHEIM"]
                    dept_level_idx = clearance_levels.index(department["security_level"])
                    security_clearance = clearance_levels[max(0, dept_level_idx - 1)]
                else:
                    security_clearance = department["security_level"]

            employee = {
                "id": emp_id,
                "employee_number": f"EN{i+1:06d}",
                "first_name": fake.first_name(),
                "last_name": fake.last_name(),
                "email": f"{emp_id.lower()}@neuroquantum-corp.de",
                "department_id": department["id"],
                "role": role,
                "security_clearance": security_clearance,
                "hire_date": fake.date_between(start_date='-10y', end_date='now').isoformat(),
                "salary": random.randint(35000, 150000),
                "phone": fake.phone_number(),
                "office_location": department["location"],
                "office_room": f"{random.choice(['A', 'B', 'C'])}{random.randint(100, 999)}",
                "manager_id": department["manager_id"] if emp_id != department["manager_id"] else None,
                "active": random.random() < 0.95,  # 95% aktiv
                "last_login": fake.date_time_between(start_date='-30d', end_date='now').isoformat(),
                "created_at": fake.date_time_between(start_date='-2y', end_date='now').isoformat()
            }

            employees.append(employee)

        self.employees = employees
        return employees

    def generate_documents(self) -> List[Dict]:
        """📄 Generiere sicherheitskritische Dokumente"""
        logger.info(f"Generating {TOTAL_DOCUMENTS} documents...")

        documents = []

        for i in range(TOTAL_DOCUMENTS):
            doc_id = f"DOC_{i+1:06d}"

            # Zufälliger Dokumenttyp
            doc_type = random.choice(self.document_types)

            # Bestimme Sicherheitslevel basierend auf Typ
            if doc_type in ["Personalakten", "Finanzberichte", "Strategische_Pläne"]:
                security_level = random.choice(["GEHEIM", "STRENG_GEHEIM"])
            elif doc_type in ["Verträge", "Compliance_Dokumente", "Sicherheitsrichtlinien"]:
                security_level = random.choice(["VERTRAULICH", "GEHEIM"])
            elif doc_type in ["Technische_Spezifikationen", "Forschung_Entwicklung"]:
                security_level = random.choice(["VERTRAULICH", "GEHEIM", "STRENG_GEHEIM"])
            else:
                security_level = random.choice(["INTERN", "VERTRAULICH"])

            # Zufälliger Ersteller
            creator = random.choice(self.employees)

            # Zufällige Abteilung (meist die des Erstellers)
            if random.random() < 0.7:
                owner_dept = next(d for d in self.departments if d["id"] == creator["department_id"])
            else:
                owner_dept = random.choice(self.departments)

            # Generiere realistische Dateigrößen
            file_size = random.choice([
                random.randint(1024, 10240),      # Kleine Dateien (1-10 KB)
                random.randint(10240, 1048576),   # Mittlere Dateien (10KB-1MB)
                random.randint(1048576, 104857600) # Große Dateien (1-100MB)
            ])

            document = {
                "id": doc_id,
                "title": f"{doc_type.replace('_', ' ')} - {fake.catch_phrase()}",
                "document_type": doc_type,
                "security_classification": security_level,
                "owner_department_id": owner_dept["id"],
                "creator_employee_id": creator["id"],
                "file_name": f"{doc_id}_{fake.file_name(extension='pdf')}",
                "file_size_bytes": file_size,
                "file_hash": hashlib.sha256(f"{doc_id}{random.random()}".encode()).hexdigest(),
                "version": f"{random.randint(1, 10)}.{random.randint(0, 9)}",
                "status": random.choice(["DRAFT", "REVIEW", "APPROVED", "ARCHIVED"]),
                "tags": random.sample([
                    "wichtig", "deadline", "review_required", "confidential",
                    "legal", "financial", "technical", "strategic"
                ], k=random.randint(1, 4)),
                "retention_period_years": random.choice([3, 5, 7, 10, 25]),
                "created_at": fake.date_time_between(start_date='-2y', end_date='now').isoformat(),
                "modified_at": fake.date_time_between(start_date='-1y', end_date='now').isoformat(),
                "expires_at": (datetime.now() + timedelta(days=random.randint(365, 3650))).isoformat(),
                "metadata": {
                    "project_code": f"PRJ_{random.randint(1, 1000):04d}",
                    "compliance_required": random.random() < 0.3,
                    "encryption_level": "AES256" if security_level in ["GEHEIM", "STRENG_GEHEIM"] else "AES128"
                }
            }

            documents.append(document)

        self.documents = documents
        return documents

    def generate_document_permissions(self) -> List[Dict]:
        """🔐 Generiere Dokumentzugriffsrechte"""
        logger.info("Generating document permissions...")

        permissions = []

        for doc in self.documents:
            # Automatische Berechtigung für Ersteller
            permissions.append({
                "document_id": doc["id"],
                "employee_id": doc["creator_employee_id"],
                "permission_type": "FULL_ACCESS",
                "granted_by": "SYSTEM",
                "granted_at": doc["created_at"],
                "expires_at": None
            })

            # Berechtigung für Abteilung
            dept_employees = [e for e in self.employees if e["department_id"] == doc["owner_department_id"]]

            for emp in dept_employees:
                # Nur Mitarbeiter mit ausreichender Sicherheitsfreigabe
                clearance_levels = ["ÖFFENTLICH", "INTERN", "VERTRAULICH", "GEHEIM", "STRENG_GEHEIM"]
                emp_level = clearance_levels.index(emp["security_clearance"])
                doc_level = clearance_levels.index(doc["security_classification"])

                if emp_level >= doc_level and random.random() < 0.8:  # 80% der berechtigten bekommen Zugriff
                    permission_type = "READ_ONLY"
                    if emp["role"] in ["Abteilungsleiter", "Teamleiter", "Projektleiter"]:
                        permission_type = random.choice(["READ_WRITE", "FULL_ACCESS"])
                    elif emp["id"] == doc["creator_employee_id"]:
                        permission_type = "FULL_ACCESS"

                    # Verwende einfache Zeitberechnung ohne Faker für granted_at
                    doc_created = datetime.fromisoformat(doc["created_at"].replace('Z', '+00:00').replace('+00:00', ''))
                    days_after_creation = random.randint(0, 30)
                    granted_time = doc_created + timedelta(days=days_after_creation)

                    permissions.append({
                        "document_id": doc["id"],
                        "employee_id": emp["id"],
                        "permission_type": permission_type,
                        "granted_by": doc["creator_employee_id"],
                        "granted_at": granted_time.isoformat(),
                        "expires_at": None if random.random() < 0.7 else (
                            datetime.now() + timedelta(days=random.randint(30, 365))
                        ).isoformat()
                    })

            # Zusätzliche projektbasierte Berechtigungen
            if random.random() < 0.3:  # 30% der Dokumente haben projektbasierte Zugriffe
                cross_dept_employees = random.sample(
                    [e for e in self.employees if e["department_id"] != doc["owner_department_id"]],
                    k=random.randint(1, min(5, len([e for e in self.employees if e["department_id"] != doc["owner_department_id"]])))
                )

                for emp in cross_dept_employees:
                    clearance_levels = ["ÖFFENTLICH", "INTERN", "VERTRAULICH", "GEHEIM", "STRENG_GEHEIM"]
                    emp_level = clearance_levels.index(emp["security_clearance"])
                    doc_level = clearance_levels.index(doc["security_classification"])

                    if emp_level >= doc_level:
                        # Verwende einfache Zeitberechnung ohne Faker
                        doc_created = datetime.fromisoformat(doc["created_at"].replace('Z', '+00:00').replace('+00:00', ''))
                        days_after_creation = random.randint(0, 60)
                        granted_time = doc_created + timedelta(days=days_after_creation)

                        permissions.append({
                            "document_id": doc["id"],
                            "employee_id": emp["id"],
                            "permission_type": "READ_ONLY",
                            "granted_by": doc["creator_employee_id"],
                            "granted_at": granted_time.isoformat(),
                            "expires_at": (datetime.now() + timedelta(days=random.randint(30, 180))).isoformat()
                        })

        return permissions

    def generate_access_logs(self) -> List[Dict]:
        """📊 Generiere Zugriffslogs"""
        logger.info(f"Generating {TOTAL_ACCESS_LOGS} access logs...")

        access_logs = []

        for i in range(TOTAL_ACCESS_LOGS):
            # Zufälliges Dokument und Mitarbeiter
            document = random.choice(self.documents)
            employee = random.choice(self.employees)

            # Bestimme ob Zugriff erlaubt war
            clearance_levels = ["ÖFFENTLICH", "INTERN", "VERTRAULICH", "GEHEIM", "STRENG_GEHEIM"]
            emp_level = clearance_levels.index(employee["security_clearance"])
            doc_level = clearance_levels.index(document["security_classification"])

            access_granted = emp_level >= doc_level and random.random() < 0.9  # 90% der berechtigten Zugriffe erfolgreich

            if not access_granted:
                result = "ACCESS_DENIED"
                reason = random.choice([
                    "INSUFFICIENT_CLEARANCE", "DOCUMENT_NOT_FOUND", "PERMISSION_EXPIRED",
                    "ACCOUNT_LOCKED", "OUTSIDE_BUSINESS_HOURS"
                ])
            else:
                result = "SUCCESS"
                reason = None

            action = random.choice([
                "VIEW", "DOWNLOAD", "EDIT", "DELETE", "SHARE", "PRINT", "COPY"
            ])

            # Edit/Delete nur bei entsprechenden Berechtigungen
            if action in ["EDIT", "DELETE"] and access_granted:
                if employee["role"] not in ["Abteilungsleiter", "Teamleiter", "Projektleiter"] and \
                   employee["id"] != document["creator_employee_id"]:
                    result = "ACCESS_DENIED"
                    reason = "INSUFFICIENT_PERMISSIONS"

            access_log = {
                "id": f"LOG_{i+1:07d}",
                "document_id": document["id"],
                "employee_id": employee["id"],
                "action": action,
                "result": result,
                "reason": reason,
                "ip_address": fake.ipv4(),
                "user_agent": fake.user_agent(),
                "session_id": str(uuid.uuid4()),
                "duration_seconds": random.randint(1, 3600) if result == "SUCCESS" else 0,
                "bytes_transferred": random.randint(1024, document["file_size_bytes"]) if action == "DOWNLOAD" and result == "SUCCESS" else 0,
                "location": random.choice(["Office", "Home", "Mobile", "External"]),
                "timestamp": fake.date_time_between(start_date='-6m', end_date='now').isoformat()
            }

            access_logs.append(access_log)

        self.access_logs = access_logs
        return access_logs

    def generate_security_events(self) -> List[Dict]:
        """🚨 Generiere Sicherheitsereignisse"""
        logger.info(f"Generating {TOTAL_SECURITY_EVENTS} security events...")

        security_events = []

        event_types = [
            "UNAUTHORIZED_ACCESS_ATTEMPT", "MULTIPLE_LOGIN_FAILURES", "PRIVILEGE_ESCALATION",
            "DATA_EXFILTRATION_ATTEMPT", "MALWARE_DETECTED", "PHISHING_ATTEMPT",
            "UNUSUAL_ACCESS_PATTERN", "AFTER_HOURS_ACCESS", "BULK_DOWNLOAD",
            "UNAUTHORIZED_FILE_SHARE", "WEAK_PASSWORD_DETECTED", "ACCOUNT_COMPROMISE"
        ]

        severity_levels = ["LOW", "MEDIUM", "HIGH", "CRITICAL"]

        for i in range(TOTAL_SECURITY_EVENTS):
            event_type = random.choice(event_types)

            # Bestimme Schweregrad basierend auf Event-Typ
            if event_type in ["DATA_EXFILTRATION_ATTEMPT", "ACCOUNT_COMPROMISE", "PRIVILEGE_ESCALATION"]:
                severity = random.choice(["HIGH", "CRITICAL"])
            elif event_type in ["UNAUTHORIZED_ACCESS_ATTEMPT", "MALWARE_DETECTED", "PHISHING_ATTEMPT"]:
                severity = random.choice(["MEDIUM", "HIGH"])
            else:
                severity = random.choice(["LOW", "MEDIUM"])

            employee = random.choice(self.employees)

            # Erstelle zusätzliche Kontext-Daten basierend auf Event-Typ
            additional_data = {}
            if event_type == "BULK_DOWNLOAD":
                additional_data = {
                    "files_downloaded": random.randint(10, 100),
                    "total_size_mb": random.randint(100, 10000)
                }
            elif event_type == "MULTIPLE_LOGIN_FAILURES":
                additional_data = {
                    "failure_count": random.randint(3, 15),
                    "time_window_minutes": random.randint(5, 60)
                }
            elif event_type == "UNUSUAL_ACCESS_PATTERN":
                additional_data = {
                    "access_count": random.randint(50, 500),
                    "unusual_hours": random.random() < 0.5
                }

            security_event = {
                "id": f"SEC_{i+1:06d}",
                "event_type": event_type,
                "severity": severity,
                "employee_id": employee["id"],
                "department_id": employee["department_id"],
                "description": f"{event_type.replace('_', ' ').title()} detected for user {employee['email']}",
                "source_ip": fake.ipv4(),
                "target_resource": random.choice(self.documents)["id"] if random.random() < 0.6 else None,
                "status": random.choice(["OPEN", "INVESTIGATING", "RESOLVED", "FALSE_POSITIVE"]),
                "assigned_to": random.choice([e["id"] for e in self.employees if e["department_id"] == "DEPT_014"]),  # Sicherheitsabteilung
                "detection_method": random.choice(["AUTOMATED", "MANUAL", "USER_REPORT", "EXTERNAL_ALERT"]),
                "risk_score": random.randint(1, 100),
                "additional_data": additional_data,
                "created_at": fake.date_time_between(start_date='-3m', end_date='now').isoformat(),
                "updated_at": fake.date_time_between(start_date='-1m', end_date='now').isoformat()
            }

            security_events.append(security_event)

        self.security_events = security_events
        return security_events

    def save_data_to_files(self):
        """💾 Speichere alle generierten Daten in JSON-Dateien"""
        logger.info("Saving generated data to files...")

        datasets = {
            "departments": self.departments,
            "employees": self.employees,
            "documents": self.documents,
            "document_permissions": self.generate_document_permissions(),
            "access_logs": self.access_logs,
            "security_events": self.security_events
        }

        for dataset_name, data in datasets.items():
            filename = f"generated_{dataset_name}.json"
            with open(filename, 'w', encoding='utf-8') as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            logger.info(f"Saved {len(data)} {dataset_name} to {filename}")

    def generate_all_data(self):
        """🚀 Generiere alle Daten"""
        logger.info("Starting enterprise data generation...")

        start_time = time.time()

        # 1. Abteilungen generieren
        self.generate_departments()

        # 2. Mitarbeiter generieren
        self.generate_employees()

        # 3. Dokumente generieren
        self.generate_documents()

        # 4. Zugriffslogs generieren
        self.generate_access_logs()

        # 5. Sicherheitsereignisse generieren
        self.generate_security_events()

        # 6. Daten speichern
        self.save_data_to_files()

        generation_time = time.time() - start_time
        total_records = (len(self.departments) + len(self.employees) +
                        len(self.documents) + len(self.access_logs) +
                        len(self.security_events))

        logger.info(f"Data generation completed in {generation_time:.2f} seconds")
        logger.info(f"Total records generated: {total_records:,}")

        return {
            "departments": len(self.departments),
            "employees": len(self.employees),
            "documents": len(self.documents),
            "access_logs": len(self.access_logs),
            "security_events": len(self.security_events),
            "total_records": total_records,
            "generation_time_seconds": generation_time
        }


class NeuroQuantumDBDataTester:
    """🧠 Tester für intelligente Abfragen auf den generierten Daten"""

    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url
        self.api_base = f"{base_url}/api/v1"
        self.api_key = None
        self.session = requests.Session()
        self.test_results = []

    def setup_api_connection(self):
        """🔌 API-Verbindung einrichten"""
        logger.info("Setting up API connection...")

        # API-Key generieren
        data = {
            "name": "enterprise-data-tester",
            "permissions": ["read", "write", "admin"]
        }

        response = self.session.post(
            f"{self.api_base}/auth/generate-key",
            json=data,
            headers={"Content-Type": "application/json"},
            timeout=30
        )

        if response.status_code == 200:
            result = response.json()
            self.api_key = result.get("api_key") or result.get("data", {}).get("api_key")
            if self.api_key:
                logger.info(f"API key generated: {self.api_key[:20]}...")
                return True

        logger.error("Failed to generate API key")
        return False

    def load_data_to_database(self):
        """📥 Lade generierte Daten in die NeuroQuantumDB"""
        logger.info("Loading data into NeuroQuantumDB...")

        if not self.api_key:
            logger.error("No API key available")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        # Lade alle Datensätze
        datasets = [
            "departments", "employees", "documents",
            "document_permissions", "access_logs", "security_events"
        ]

        for dataset in datasets:
            try:
                with open(f"generated_{dataset}.json", 'r', encoding='utf-8') as f:
                    data = json.load(f)

                logger.info(f"Loading {len(data)} {dataset} records...")

                # Daten in Batches laden
                for i in range(0, len(data), BATCH_SIZE):
                    batch = data[i:i+BATCH_SIZE]

                    load_data = {
                        "table": dataset,
                        "data": batch,
                        "mode": "insert",
                        "compression": "dna" if len(batch) > 100 else None
                    }

                    response = self.session.post(
                        f"{self.api_base}/data/load",
                        json=load_data,
                        headers=headers,
                        timeout=60
                    )

                    if response.status_code == 200:
                        logger.info(f"Loaded batch {i//BATCH_SIZE + 1} of {dataset}")
                    else:
                        logger.error(f"Failed to load batch {i//BATCH_SIZE + 1} of {dataset}")

            except FileNotFoundError:
                logger.warning(f"File generated_{dataset}.json not found")
                continue

        return True

    def test_neuromorphic_security_queries(self):
        """🧠 Teste neuromorphe Sicherheitsabfragen"""
        logger.info("Testing neuromorphic security queries...")

        queries = [
            # 1. Verdächtige Zugriffsmuster erkennen
            {
                "name": "Suspicious Access Patterns",
                "query": """
                    NEUROMATCH access_logs 
                    WHERE action IN ('DOWNLOAD', 'COPY', 'PRINT') 
                    AND location = 'External'
                    AND timestamp > NOW() - INTERVAL 30 DAY
                    WITH SYNAPTIC_WEIGHT 0.9, PLASTICITY_RATE 0.05
                """,
                "learning_enabled": True,
                "plasticity_threshold": 0.7
            },

            # 2. Mitarbeiter mit ungewöhnlich hohen Sicherheitsberechtigungen
            {
                "name": "High Privilege Users",
                "query": """
                    NEUROMATCH employees e
                    JOIN departments d ON e.department_id = d.id
                    WHERE e.security_clearance = 'STRENG_GEHEIM'
                    AND e.role NOT IN ('Abteilungsleiter', 'Geschäftsführung')
                    WITH SYNAPTIC_WEIGHT 0.85, MEMORY_CONSOLIDATION true
                """,
                "learning_enabled": True
            },

            # 3. Dokumente mit kritischen Sicherheitsereignissen
            {
                "name": "Critical Document Security Events",
                "query": """
                    NEUROMATCH documents d
                    JOIN security_events s ON d.id = s.target_resource
                    WHERE d.security_classification IN ('GEHEIM', 'STRENG_GEHEIM')
                    AND s.severity IN ('HIGH', 'CRITICAL')
                    AND s.status = 'OPEN'
                    WITH SYNAPTIC_WEIGHT 0.95, PATTERN_RECOGNITION advanced
                """,
                "learning_enabled": True,
                "pattern_analysis": True
            }
        ]

        for query_test in queries:
            try:
                response = self.session.post(
                    f"{self.api_base}/neuromorphic/query",
                    json=query_test,
                    headers={
                        "Content-Type": "application/json",
                        "X-API-Key": self.api_key
                    },
                    timeout=60
                )

                if response.status_code == 200:
                    result = response.json()
                    logger.info(f"✅ {query_test['name']}: {result.get('rows_returned', 0)} results")
                else:
                    logger.error(f"❌ {query_test['name']}: Failed")

            except Exception as e:
                logger.error(f"❌ {query_test['name']}: {str(e)}")

    def test_quantum_optimization_queries(self):
        """⚛️ Teste Quantum-Optimierungsabfragen"""
        logger.info("Testing quantum optimization queries...")

        queries = [
            # 1. Optimale Dokumentzugriffe für Projekte
            {
                "name": "Project Document Access Optimization",
                "query": """
                    QUANTUM_SELECT d.id, d.title, e.name, dp.permission_type
                    FROM documents d
                    JOIN document_permissions dp ON d.id = dp.document_id
                    JOIN employees e ON dp.employee_id = e.id
                    WHERE d.metadata->>'project_code' LIKE 'PRJ_%'
                    AND dp.expires_at IS NULL OR dp.expires_at > NOW()
                    WITH GROVER_ITERATIONS 20, AMPLITUDE_AMPLIFICATION true
                """,
                "optimization_target": "minimize_access_time",
                "parallel_processing": True
            },

            # 2. Sicherheitsrisiko-Optimierung
            {
                "name": "Security Risk Optimization",
                "query": """
                    QUANTUM_SELECT e.id, e.security_clearance, 
                                   COUNT(al.id) as access_count,
                                   COUNT(se.id) as security_events
                    FROM employees e
                    LEFT JOIN access_logs al ON e.id = al.employee_id
                    LEFT JOIN security_events se ON e.id = se.employee_id
                    WHERE al.timestamp > NOW() - INTERVAL 7 DAY
                    GROUP BY e.id, e.security_clearance
                    HAVING COUNT(se.id) > 0
                    WITH QUANTUM_ANNEALING 1500, ENERGY_MINIMIZATION true
                """,
                "optimization_target": "minimize_security_risk"
            },

            # 3. Ressourcen-Allokation für Abteilungen
            {
                "name": "Department Resource Allocation",
                "query": """
                    QUANTUM_SELECT d.name, d.budget, d.employee_count,
                                   AVG(e.salary) as avg_salary,
                                   COUNT(doc.id) as document_count
                    FROM departments d
                    JOIN employees e ON d.id = e.department_id
                    LEFT JOIN documents doc ON d.id = doc.owner_department_id
                    GROUP BY d.id, d.name, d.budget, d.employee_count
                    WITH QUANTUM_OPTIMIZATION efficiency, PARALLEL_QUBITS 8
                """,
                "optimization_target": "maximize_efficiency"
            }
        ]

        for query_test in queries:
            try:
                response = self.session.post(
                    f"{self.api_base}/quantum/search",
                    json=query_test,
                    headers={
                        "Content-Type": "application/json",
                        "X-API-Key": self.api_key
                    },
                    timeout=60
                )

                if response.status_code == 200:
                    result = response.json()
                    speedup = result.get('quantum_speedup', 0)
                    logger.info(f"✅ {query_test['name']}: {speedup}x speedup")
                else:
                    logger.error(f"❌ {query_test['name']}: Failed")

            except Exception as e:
                logger.error(f"❌ {query_test['name']}: {str(e)}")

    def test_dna_compression_queries(self):
        """🧬 Teste DNA-Speicher für große Datensätze"""
        logger.info("Testing DNA compression for large datasets...")

        # Komprimiere Zugriffslogs mit DNA-Speicher
        try:
            with open("generated_access_logs.json", 'r', encoding='utf-8') as f:
                access_logs = json.load(f)

            # Start with a smaller sample to avoid payload size limits
            sample_size = 100  # Reduced from 10,000 to 100
            sample_logs = random.sample(access_logs, min(sample_size, len(access_logs)))

            # Create a more compact data structure for compression
            compact_logs = []
            for log in sample_logs:
                compact_log = {
                    "id": log["id"],
                    "doc_id": log["document_id"],
                    "emp_id": log["employee_id"],
                    "action": log["action"],
                    "result": log["result"],
                    "timestamp": log["timestamp"],
                    "duration": log.get("duration_seconds", 0),
                    "bytes": log.get("bytes_transferred", 0)
                }
                compact_logs.append(compact_log)

            compression_data = {
                "data": json.dumps(compact_logs),
                "compression_level": 5,  # Reduced from 9 to 5 for faster processing
                "error_correction": True,
                "biological_patterns": True,
                "metadata": {
                    "data_type": "access_logs",
                    "record_count": len(compact_logs),
                    "compression_target": "long_term_storage"
                }
            }

            # Check payload size before sending
            payload_size = len(json.dumps(compression_data).encode('utf-8'))
            logger.info(f"DNA compression payload size: {payload_size} bytes")

            if payload_size > 1024 * 1024:  # 1MB limit
                logger.warning(f"Payload too large ({payload_size} bytes), reducing sample size further")
                sample_size = 50
                compact_logs = compact_logs[:sample_size]
                compression_data["data"] = json.dumps(compact_logs)
                compression_data["metadata"]["record_count"] = len(compact_logs)

            response = self.session.post(
                f"{self.api_base}/dna/compress",
                json=compression_data,
                headers={
                    "Content-Type": "application/json",
                    "X-API-Key": self.api_key
                },
                timeout=120
            )

            if response.status_code == 200:
                result = response.json()
                ratio = result.get("compression_ratio", 0)
                logger.info(f"✅ DNA Compression: {ratio}:1 ratio for {len(compact_logs)} access logs")

                # Teste Decompression
                if "dna_sequence" in result:
                    decompression_data = {
                        "dna_sequence": result["dna_sequence"],
                        "verify_integrity": True
                    }

                    decomp_response = self.session.post(
                        f"{self.api_base}/dna/decompress",
                        json=decompression_data,
                        headers={
                            "Content-Type": "application/json",
                            "X-API-Key": self.api_key
                        },
                        timeout=120
                    )

                    if decomp_response.status_code == 200:
                        logger.info("✅ DNA Decompression: Successfully restored data")
                    else:
                        logger.error(f"❌ DNA Decompression: Failed with status {decomp_response.status_code}")
            elif response.status_code == 413:
                logger.error("❌ DNA Compression: Payload too large - trying with even smaller sample")

                # Try with minimal sample
                minimal_logs = compact_logs[:10]
                minimal_data = {
                    "data": json.dumps(minimal_logs),
                    "compression_level": 3,
                    "error_correction": False,
                    "biological_patterns": False,
                    "metadata": {
                        "data_type": "access_logs",
                        "record_count": len(minimal_logs),
                        "compression_target": "test"
                    }
                }

                minimal_response = self.session.post(
                    f"{self.api_base}/dna/compress",
                    json=minimal_data,
                    headers={
                        "Content-Type": "application/json",
                        "X-API-Key": self.api_key
                    },
                    timeout=60
                )

                if minimal_response.status_code == 200:
                    logger.info(f"✅ DNA Compression (minimal): Success with {len(minimal_logs)} records")
                else:
                    logger.error(f"❌ DNA Compression (minimal): Failed with status {minimal_response.status_code}")
            else:
                logger.error(f"❌ DNA Compression: Failed with status {response.status_code}")
                if response.text:
                    logger.error(f"Response: {response.text[:200]}")

        except Exception as e:
            logger.error(f"❌ DNA Compression Test: {str(e)}")

    def test_complex_business_intelligence_queries(self):
        """📊 Teste komplexe Business Intelligence Abfragen"""
        logger.info("Testing complex business intelligence queries...")

        queries = [
            # 1. Sicherheitsrisiko-Dashboard
            {
                "name": "Security Risk Dashboard",
                "query": """
                    WITH high_risk_employees AS (
                        NEUROMATCH employees e
                        JOIN security_events se ON e.id = se.employee_id
                        WHERE se.severity IN ('HIGH', 'CRITICAL')
                        AND se.created_at > NOW() - INTERVAL 30 DAY
                        WITH SYNAPTIC_WEIGHT 0.9
                    ),
                    document_access_patterns AS (
                        QUANTUM_SELECT al.employee_id, al.document_id, 
                                       COUNT(*) as access_frequency,
                                       al.action
                        FROM access_logs al
                        WHERE al.timestamp > NOW() - INTERVAL 7 DAY
                        GROUP BY al.employee_id, al.document_id, al.action
                        WITH GROVER_ITERATIONS 15
                    )
                    SELECT hre.id, hre.name, hre.department_id,
                           dap.access_frequency, 
                           CASE 
                               WHEN dap.access_frequency > 50 THEN 'HIGH_ACTIVITY'
                               WHEN dap.access_frequency > 20 THEN 'MEDIUM_ACTIVITY'
                               ELSE 'LOW_ACTIVITY'
                           END as activity_level
                    FROM high_risk_employees hre
                    LEFT JOIN document_access_patterns dap ON hre.id = dap.employee_id
                    ORDER BY dap.access_frequency DESC
                """,
                "hybrid_processing": True,
                "optimization_level": "aggressive"
            },

            # 2. Compliance-Überwachung
            {
                "name": "Compliance Monitoring",
                "query": """
                    NEUROMATCH documents d
                    JOIN employees e ON d.creator_employee_id = e.id
                    JOIN departments dept ON e.department_id = dept.id
                    WHERE d.metadata->>'compliance_required' = 'true'
                    AND d.expires_at < NOW() + INTERVAL 90 DAY
                    AND d.status != 'ARCHIVED'
                    WITH SYNAPTIC_WEIGHT 0.88, COMPLIANCE_PATTERN_DETECTION true
                """,
                "learning_enabled": True,
                "compliance_mode": True
            },

            # 3. Abteilungsübergreifende Dokumentzugriffe
            {
                "name": "Cross-Department Document Access",
                "query": """
                    QUANTUM_SELECT dept1.name as creator_dept,
                                   dept2.name as accessor_dept,
                                   d.security_classification,
                                   COUNT(*) as access_count,
                                   AVG(al.duration_seconds) as avg_duration
                    FROM access_logs al
                    JOIN employees e1 ON al.employee_id = e1.id
                    JOIN departments dept2 ON e1.department_id = dept2.id
                    JOIN documents d ON al.document_id = d.id
                    JOIN employees e2 ON d.creator_employee_id = e2.id
                    JOIN departments dept1 ON e2.department_id = dept1.id
                    WHERE dept1.id != dept2.id
                    AND al.result = 'SUCCESS'
                    AND al.timestamp > NOW() - INTERVAL 30 DAY
                    GROUP BY dept1.name, dept2.name, d.security_classification
                    HAVING COUNT(*) > 5
                    WITH QUANTUM_ANALYSIS cross_department_patterns
                """,
                "pattern_analysis": "cross_department",
                "security_analysis": True
            }
        ]

        for query_test in queries:
            try:
                # Wähle Endpoint basierend auf Query-Typ
                endpoint = "/neuromorphic/query"
                if "QUANTUM_SELECT" in query_test["query"] or query_test.get("pattern_analysis"):
                    endpoint = "/quantum/search"

                response = self.session.post(
                    f"{self.api_base}{endpoint}",
                    json=query_test,
                    headers={
                        "Content-Type": "application/json",
                        "X-API-Key": self.api_key
                    },
                    timeout=120
                )

                if response.status_code == 200:
                    result = response.json()
                    rows = result.get('rows_returned', result.get('results_count', 0))
                    exec_time = result.get('execution_time_us', result.get('execution_time_ms', 0))
                    logger.info(f"✅ {query_test['name']}: {rows} results, {exec_time}μs")
                else:
                    logger.error(f"❌ {query_test['name']}: HTTP {response.status_code}")

            except Exception as e:
                logger.error(f"❌ {query_test['name']}: {str(e)}")

    def run_training_scenarios(self):
        """🎓 Trainiere das System mit realistischen Szenarien"""
        logger.info("Running training scenarios...")

        # 1. Sicherheitsmuster-Training
        security_patterns = [
            {
                "pattern": ["UNAUTHORIZED_ACCESS_ATTEMPT", "MULTIPLE_LOGIN_FAILURES", "ACCOUNT_COMPROMISE"],
                "weight": 0.95,
                "label": "account_takeover_attack"
            },
            {
                "pattern": ["BULK_DOWNLOAD", "AFTER_HOURS_ACCESS", "EXTERNAL_LOCATION"],
                "weight": 0.90,
                "label": "data_exfiltration_attempt"
            },
            {
                "pattern": ["PRIVILEGE_ESCALATION", "UNUSUAL_ACCESS_PATTERN", "HIGH_CLEARANCE_DOCUMENT"],
                "weight": 0.88,
                "label": "insider_threat"
            },
            {
                "pattern": ["PHISHING_ATTEMPT", "WEAK_PASSWORD_DETECTED", "MALWARE_DETECTED"],
                "weight": 0.85,
                "label": "compromise_precursor"
            }
        ]

        training_data = {
            "training_data": security_patterns,
            "learning_rate": 0.025,
            "epochs": 100,
            "training_type": "security_pattern_recognition",
            "validation_split": 0.2
        }

        try:
            response = self.session.post(
                f"{self.api_base}/neuromorphic/train",
                json=training_data,
                headers={
                    "Content-Type": "application/json",
                    "X-API-Key": self.api_key
                },
                timeout=180
            )

            if response.status_code == 200:
                logger.info("✅ Security pattern training completed")
            else:
                logger.error("❌ Security pattern training failed")

        except Exception as e:
            logger.error(f"❌ Training error: {str(e)}")

    def run_all_tests(self):
        """🚀 Führe alle Tests aus"""
        logger.info("Starting comprehensive NeuroQuantumDB enterprise tests...")

        if not self.setup_api_connection():
            logger.error("Cannot proceed without API connection")
            return False

        # Lade Daten in die Datenbank
        self.load_data_to_database()

        # Warte kurz für Datenverarbeitung
        time.sleep(5)

        # Führe Tests aus
        self.test_neuromorphic_security_queries()
        self.test_quantum_optimization_queries()
        self.test_dna_compression_queries()
        self.test_complex_business_intelligence_queries()
        self.run_training_scenarios()

        logger.info("All enterprise tests completed!")
        return True


def main():
    """🎯 Hauptfunktion"""
    parser = argparse.ArgumentParser(description="NeuroQuantumDB Enterprise Data Generator & Tester")
    parser.add_argument("--generate", "-g", action="store_true", help="Generate enterprise data")
    parser.add_argument("--test", "-t", action="store_true", help="Run database tests")
    parser.add_argument("--all", "-a", action="store_true", help="Generate data and run tests")
    parser.add_argument("--url", default="http://localhost:8080", help="Database URL")

    args = parser.parse_args()

    if args.generate or args.all:
        # Generiere Daten
        generator = EnterpriseDataGenerator()
        stats = generator.generate_all_data()

        print("\n" + "="*60)
        print("🏢 ENTERPRISE DATA GENERATION COMPLETE")
        print("="*60)
        print(f"📊 Departments: {stats['departments']:,}")
        print(f"👥 Employees: {stats['employees']:,}")
        print(f"📄 Documents: {stats['documents']:,}")
        print(f"📊 Access Logs: {stats['access_logs']:,}")
        print(f"🚨 Security Events: {stats['security_events']:,}")
        print(f"🔢 Total Records: {stats['total_records']:,}")
        print(f"⏱️  Generation Time: {stats['generation_time_seconds']:.2f}s")
        print("="*60)

    if args.test or args.all:
        # Führe Tests aus
        tester = NeuroQuantumDBDataTester(args.url)
        tester.run_all_tests()


if __name__ == "__main__":
    main()
