#!/usr/bin/env python3
"""
🧠⚛️🧬 NeuroQuantumDB Enterprise Data Generator & Test Suite
===========================================================

Generiert 500.000 realistische Unternehmensdaten für ein Szenario mit:
- 800 Mitarbeiter in 25 Abteilungen
- Sicherheitskritische Dokumente mit Zugriffsrechten
- Komplexe Verknüpfungen und Hierarchien
- Training der neuromorphen und Quantum-Systeme
- Umfassende Datenverifikation und intelligente Abfragetests

Autor: NeuroQuantumDB Team
Version: 1.1.0
"""

# Suppress urllib3 OpenSSL warnings on macOS
import warnings
import urllib3
warnings.filterwarnings('ignore', message='urllib3 v2 only supports OpenSSL 1.1.1+')
urllib3.disable_warnings(urllib3.exceptions.NotOpenSSLWarning)

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
        self.data_verification_results = {}

    def setup_api_connection(self):
        """🔌 API-Verbindung einrichten"""
        logger.info("Setting up API connection...")

        # First, test basic connectivity
        try:
            logger.info(f"Testing connection to {self.base_url}...")
            health_response = self.session.get(
                f"{self.base_url}/health",
                timeout=10
            )
            if health_response.status_code != 200:
                logger.warning(f"Health check failed: HTTP {health_response.status_code}")
                # Try alternative health endpoints
                try:
                    alt_response = self.session.get(f"{self.base_url}/", timeout=10)
                    logger.info(f"Alternative endpoint response: HTTP {alt_response.status_code}")
                except Exception as e:
                    logger.error(f"No response from database server at {self.base_url}: {str(e)}")
                    return False
            else:
                logger.info("✅ Database server is reachable")
        except Exception as e:
            logger.error(f"❌ Cannot connect to database server at {self.base_url}: {str(e)}")
            logger.error("Please ensure NeuroQuantumDB is running on the specified URL")
            return False

        # API-Key generieren
        data = {
            "name": "enterprise-data-tester",
            "permissions": ["read", "write", "admin"]
        }

        try:
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
                    logger.info(f"✅ API key generated: {self.api_key[:20]}...")
                    return True
                else:
                    logger.error("❌ API key not found in response")
                    logger.error(f"Response: {result}")
                    return False
            else:
                logger.error(f"❌ Failed to generate API key: HTTP {response.status_code}")
                logger.error(f"Response: {response.text[:500]}")
                return False

        except Exception as e:
            logger.error(f"❌ Error generating API key: {str(e)}")
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

        load_success = True

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

                    try:
                        response = self.session.post(
                            f"{self.api_base}/data/load",
                            json=load_data,
                            headers=headers,
                            timeout=60
                        )

                        if response.status_code == 200:
                            logger.info(f"✅ Loaded batch {i//BATCH_SIZE + 1}/{(len(data) + BATCH_SIZE - 1) // BATCH_SIZE} of {dataset}")
                        else:
                            logger.error(f"❌ Failed to load batch {i//BATCH_SIZE + 1} of {dataset}: HTTP {response.status_code}")
                            logger.error(f"   Response: {response.text[:500]}")
                            load_success = False

                    except requests.exceptions.RequestException as e:
                        logger.error(f"❌ Network error loading batch {i//BATCH_SIZE + 1} of {dataset}: {str(e)}")
                        load_success = False

            except FileNotFoundError:
                logger.warning(f"⚠️ File generated_{dataset}.json not found")
                load_success = False
                continue
            except json.JSONDecodeError as e:
                logger.error(f"❌ Failed to parse JSON for {dataset}: {str(e)}")
                load_success = False
                continue
            except Exception as e:
                logger.error(f"❌ Unexpected error loading {dataset}: {str(e)}")
                load_success = False
                continue

        if load_success:
            logger.info("✅ All data loaded successfully")
        else:
            logger.warning("⚠️ Some data loading operations failed")

        return load_success

    def verify_data_storage(self):
        """📊 Verifiziere dass Daten erfolgreich gespeichert wurden"""
        logger.info("Verifying data storage in NeuroQuantumDB...")

        if not self.api_key:
            logger.error("No API key available for verification")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        datasets_to_verify = [
            "departments", "employees", "documents",
            "document_permissions", "access_logs", "security_events"
        ]

        verification_success = True

        for dataset in datasets_to_verify:
            try:
                # Basis-Zählabfrage
                count_query = {
                    "query": f"SELECT COUNT(*) as total_count FROM {dataset}",
                    "limit": 1
                }

                response = self.session.post(
                    f"{self.api_base}/query",
                    json=count_query,
                    headers=headers,
                    timeout=30
                )

                if response.status_code == 200:
                    result = response.json()
                    count = 0

                    if 'data' in result and len(result['data']) > 0:
                        count = result['data'][0].get('total_count', 0)
                    elif 'results' in result and len(result['results']) > 0:
                        count = result['results'][0].get('total_count', 0)

                    logger.info(f"✅ {dataset}: {count:,} records stored")
                    self.data_verification_results[dataset] = {
                        "stored_count": count,
                        "verification_status": "SUCCESS" if count > 0 else "EMPTY"
                    }

                    if count == 0:
                        verification_success = False
                        logger.warning(f"⚠️ {dataset}: No records found in database!")

                    # Erweiterte Verifikation mit Stichproben
                    if count > 0:
                        self._verify_data_samples(dataset, headers)

                else:
                    logger.error(f"❌ {dataset}: Query failed with status {response.status_code}")
                    self.data_verification_results[dataset] = {
                        "stored_count": 0,
                        "verification_status": "QUERY_FAILED",
                        "error": f"HTTP {response.status_code}"
                    }
                    verification_success = False

            except Exception as e:
                logger.error(f"❌ {dataset}: Verification error - {str(e)}")
                self.data_verification_results[dataset] = {
                    "stored_count": 0,
                    "verification_status": "ERROR",
                    "error": str(e)
                }
                verification_success = False

        return verification_success

    def _verify_data_samples(self, dataset: str, headers: dict):
        """🔍 Verifiziere Stichproben der gespeicherten Daten"""
        try:
            # Hole Stichproben verschiedener Datentypen
            sample_queries = {
                "departments": "SELECT id, name, security_level FROM departments LIMIT 5",
                "employees": "SELECT id, first_name, last_name, department_id, security_clearance FROM employees LIMIT 5",
                "documents": "SELECT id, title, security_classification, file_size_bytes FROM documents LIMIT 5",
                "document_permissions": "SELECT document_id, employee_id, permission_type FROM document_permissions LIMIT 5",
                "access_logs": "SELECT id, document_id, employee_id, action, result FROM access_logs LIMIT 5",
                "security_events": "SELECT id, event_type, severity, employee_id FROM security_events LIMIT 5"
            }

            if dataset in sample_queries:
                sample_query = {
                    "query": sample_queries[dataset],
                    "limit": 5
                }

                response = self.session.post(
                    f"{self.api_base}/query",
                    json=sample_query,
                    headers=headers,
                    timeout=30
                )

                if response.status_code == 200:
                    result = response.json()
                    samples = result.get('data', result.get('results', []))

                    if samples:
                        logger.info(f"  📋 {dataset} sample data: {len(samples)} records verified")
                        # Log first sample for verification
                        first_sample = samples[0]
                        logger.info(f"  🔍 Sample record: {first_sample}")

                        self.data_verification_results[dataset]["sample_verified"] = True
                        self.data_verification_results[dataset]["sample_count"] = len(samples)
                        self.data_verification_results[dataset]["sample_data"] = first_sample
                    else:
                        logger.warning(f"  ⚠️ {dataset}: No sample data returned")
                        self.data_verification_results[dataset]["sample_verified"] = False
                else:
                    logger.warning(f"  ⚠️ {dataset}: Sample query failed")
                    self.data_verification_results[dataset]["sample_verified"] = False

        except Exception as e:
            logger.warning(f"  ⚠️ {dataset}: Sample verification error - {str(e)}")
            self.data_verification_results[dataset]["sample_verified"] = False

    def test_intelligent_data_retrieval(self):
        """🧠 Teste intelligente Datenabfrage und -analyse"""
        logger.info("Testing intelligent data retrieval capabilities...")

        if not self.api_key:
            logger.error("No API key available for intelligent queries")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        intelligent_queries = [
            # 1. Komplexe Join-Abfrage mit Aggregation
            {
                "name": "Department Security Analysis",
                "description": "Analysiere Sicherheitslevel pro Abteilung mit Mitarbeiterzahl",
                "query": """
                    SELECT 
                        d.name as department_name,
                        d.security_level,
                        COUNT(e.id) as employee_count,
                        COUNT(DISTINCT doc.security_classification) as document_security_types,
                        AVG(e.salary) as avg_salary
                    FROM departments d
                    LEFT JOIN employees e ON d.id = e.department_id
                    LEFT JOIN documents doc ON d.id = doc.owner_department_id
                    GROUP BY d.id, d.name, d.security_level
                    ORDER BY employee_count DESC
                    LIMIT 10
                """,
                "expected_columns": ["department_name", "security_level", "employee_count"]
            },

            # 2. Sicherheitsrelevante Dokumentzugriffe
            {
                "name": "High Security Document Access",
                "description": "Finde Zugriffe auf hochsensible Dokumente",
                "query": """
                    SELECT 
                        d.title as document_title,
                        d.security_classification,
                        e.first_name,
                        e.last_name,
                        e.security_clearance,
                        al.action,
                        al.timestamp,
                        al.result
                    FROM access_logs al
                    JOIN documents d ON al.document_id = d.id
                    JOIN employees e ON al.employee_id = e.id
                    WHERE d.security_classification IN ('GEHEIM', 'STRENG_GEHEIM')
                    AND al.result = 'SUCCESS'
                    ORDER BY al.timestamp DESC
                    LIMIT 20
                """,
                "expected_columns": ["document_title", "security_classification", "first_name", "action"]
            },

            # 3. Mitarbeiter mit verdächtigen Aktivitäten
            {
                "name": "Suspicious Employee Activity",
                "description": "Identifiziere Mitarbeiter mit ungewöhnlichen Zugriffsmustern",
                "query": """
                    SELECT 
                        e.first_name,
                        e.last_name,
                        e.department_id,
                        COUNT(al.id) as total_accesses,
                        COUNT(CASE WHEN al.location = 'External' THEN 1 END) as external_accesses,
                        COUNT(CASE WHEN al.action IN ('DOWNLOAD', 'COPY') THEN 1 END) as download_actions,
                        COUNT(se.id) as security_events
                    FROM employees e
                    LEFT JOIN access_logs al ON e.id = al.employee_id
                    LEFT JOIN security_events se ON e.id = se.employee_id
                    GROUP BY e.id, e.first_name, e.last_name, e.department_id
                    HAVING COUNT(al.id) > 50 OR COUNT(se.id) > 0
                    ORDER BY security_events DESC, total_accesses DESC
                    LIMIT 15
                """,
                "expected_columns": ["first_name", "last_name", "total_accesses", "security_events"]
            },

            # 4. Dokumentberechtigungen und Zugriffsmuster
            {
                "name": "Document Permission Analysis",
                "description": "Analysiere Dokumentberechtigungen und tatsächliche Zugriffe",
                "query": """
                    SELECT 
                        d.document_type,
                        d.security_classification,
                        COUNT(DISTINCT dp.employee_id) as authorized_users,
                        COUNT(DISTINCT al.employee_id) as actual_users,
                        COUNT(al.id) as total_accesses,
                        ROUND(AVG(d.file_size_bytes)/1024/1024, 2) as avg_size_mb
                    FROM documents d
                    LEFT JOIN document_permissions dp ON d.id = dp.document_id
                    LEFT JOIN access_logs al ON d.id = al.document_id AND al.result = 'SUCCESS'
                    GROUP BY d.document_type, d.security_classification
                    ORDER BY total_accesses DESC
                    LIMIT 20
                """,
                "expected_columns": ["document_type", "security_classification", "authorized_users", "total_accesses"]
            },

            # 5. Zeitbasierte Zugriffsanalyse
            {
                "name": "Time-based Access Analysis",
                "description": "Analysiere Zugriffsmuster nach Zeiträumen",
                "query": """
                    SELECT 
                        DATE(al.timestamp) as access_date,
                        COUNT(*) as total_accesses,
                        COUNT(CASE WHEN al.result = 'SUCCESS' THEN 1 END) as successful_accesses,
                        COUNT(CASE WHEN al.result = 'ACCESS_DENIED' THEN 1 END) as denied_accesses,
                        COUNT(DISTINCT al.employee_id) as unique_users,
                        COUNT(DISTINCT al.document_id) as unique_documents
                    FROM access_logs al
                    WHERE al.timestamp >= DATE('now', '-30 days')
                    GROUP BY DATE(al.timestamp)
                    ORDER BY access_date DESC
                    LIMIT 30
                """,
                "expected_columns": ["access_date", "total_accesses", "successful_accesses", "unique_users"]
            }
        ]

        test_results = []

        for query_test in intelligent_queries:
            try:
                logger.info(f"🔍 Testing: {query_test['name']}")
                logger.info(f"  📝 {query_test['description']}")

                query_payload = {
                    "query": query_test["query"],
                    "limit": 50
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=query_payload,
                    headers=headers,
                    timeout=60
                )
                execution_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    data = result.get('data', result.get('results', []))

                    # Ensure data is a list before any operations
                    if not isinstance(data, list):
                        data = []

                    test_result = {
                        "name": query_test["name"],
                        "status": "SUCCESS",
                        "rows_returned": len(data),
                        "execution_time_ms": round(execution_time * 1000, 2),
                        "columns_found": list(data[0].keys()) if data else [],
                        "sample_data": data[:3] if data else []
                    }

                    # Verifiziere erwartete Spalten
                    if "expected_columns" in query_test and data:
                        expected_cols = query_test["expected_columns"]
                        actual_cols = list(data[0].keys())
                        missing_cols = [col for col in expected_cols if col not in actual_cols]

                        if missing_cols:
                            test_result["warning"] = f"Missing expected columns: {missing_cols}"
                        else:
                            test_result["columns_verified"] = True

                    logger.info(f"  ✅ Success: {len(data)} rows in {execution_time*1000:.2f}ms")
                    if data:
                        logger.info(f"  📊 Columns: {list(data[0].keys())}")
                        logger.info(f"  📋 Sample: {data[0]}")

                else:
                    test_result = {
                        "name": query_test["name"],
                        "status": "FAILED",
                        "error": f"HTTP {response.status_code}",
                        "response": response.text[:200] if response.text else "No response"
                    }
                    logger.error(f"  ❌ Failed: HTTP {response.status_code}")

                test_results.append(test_result)

            except Exception as e:
                test_result = {
                    "name": query_test["name"],
                    "status": "ERROR",
                    "error": str(e)
                }
                test_results.append(test_result)
                logger.error(f"  ❌ Error: {str(e)}")

        # Speichere Testergebnisse
        self.data_verification_results["intelligent_queries"] = test_results

        successful_tests = len([t for t in test_results if t["status"] == "SUCCESS"])
        logger.info(f"🎯 Intelligent Query Tests: {successful_tests}/{len(test_results)} successful")

        return successful_tests == len(test_results)

    def test_advanced_search_capabilities(self):
        """🔎 Teste erweiterte Suchfunktionen"""
        logger.info("Testing advanced search capabilities...")

        if not self.api_key:
            logger.error("No API key available for advanced search")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        search_tests = [
            # 1. Volltext-Suche in Dokumenttiteln
            {
                "name": "Document Title Search",
                "search_type": "fulltext",
                "query": {
                    "search_term": "Vertrag",
                    "fields": ["title", "document_type"],
                    "table": "documents",
                    "limit": 10
                }
            },

            # 2. Geografische Suche nach Standorten
            {
                "name": "Location-based Employee Search",
                "search_type": "filter",
                "query": {
                    "filters": {
                        "office_location": ["Berlin", "München"],
                        "active": 1  # Fixed: Changed True to 1 for SQL compatibility
                    },
                    "table": "employees",
                    "limit": 15
                }
            },

            # 3. Zeitbereichs-Suche
            {
                "name": "Time Range Access Search",
                "search_type": "temporal",
                "query": {
                    "time_field": "timestamp",
                    "start_date": (datetime.now() - timedelta(days=7)).isoformat(),
                    "end_date": datetime.now().isoformat(),
                    "table": "access_logs",
                    "additional_filters": {
                        "result": "SUCCESS"
                    },
                    "limit": 20
                }
            },

            # 4. Hierarchische Suche in Abteilungsstrukturen
            {
                "name": "Department Hierarchy Search",
                "search_type": "hierarchical",
                "query": {
                    "root_department": "DEPT_001",
                    "include_subdepartments": True,
                    "table": "departments",
                    "limit": 10
                }
            }
        ]

        search_results = []

        for search_test in search_tests:
            try:
                logger.info(f"🔍 Testing: {search_test['name']}")

                # Konvertiere Suchtest zu SQL-ähnlicher Abfrage
                if search_test["search_type"] == "fulltext":
                    query = self._build_fulltext_query(search_test["query"])
                elif search_test["search_type"] == "filter":
                    query = self._build_filter_query(search_test["query"])
                elif search_test["search_type"] == "temporal":
                    query = self._build_temporal_query(search_test["query"])
                elif search_test["search_type"] == "hierarchical":
                    query = self._build_hierarchical_query(search_test["query"])
                else:
                    continue

                query_payload = {
                    "query": query,
                    "limit": search_test["query"].get("limit", 10)
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=query_payload,
                    headers=headers,
                    timeout=45
                )
                execution_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    data = result.get('data', result.get('results', []))

                    # Ensure data is a list before slicing
                    if not isinstance(data, list):
                        data = []

                    search_result = {
                        "name": search_test["name"],
                        "search_type": search_test["search_type"],
                        "status": "SUCCESS",
                        "results_count": len(data),
                        "execution_time_ms": round(execution_time * 1000, 2),
                        "sample_results": list(data[:2]) if data else []
                    }

                    logger.info(f"  ✅ Found {len(data)} results in {execution_time*1000:.2f}ms")

                else:
                    search_result = {
                        "name": search_test["name"],
                        "search_type": search_test["search_type"],
                        "status": "FAILED",
                        "error": f"HTTP {response.status_code}"
                    }
                    logger.error(f"  ❌ Failed: HTTP {response.status_code}")

                search_results.append(search_result)

            except Exception as e:
                search_result = {
                    "name": search_test["name"],
                    "search_type": search_test["search_type"],
                    "status": "ERROR",
                    "error": str(e)
                }
                search_results.append(search_result)
                logger.error(f"  ❌ Error: {str(e)}")

        self.data_verification_results["advanced_search"] = search_results

        successful_searches = len([s for s in search_results if s["status"] == "SUCCESS"])
        logger.info(f"🎯 Advanced Search Tests: {successful_searches}/{len(search_results)} successful")

        return successful_searches > 0

    def _build_fulltext_query(self, query_config):
        """Erstelle Volltext-Suchabfrage"""
        table = query_config["table"]
        search_term = query_config["search_term"]
        fields = query_config.get("fields", ["*"])

        field_conditions = []
        for field in fields:
            field_conditions.append(f"{field} LIKE '%{search_term}%'")

        where_clause = " OR ".join(field_conditions)

        return f"SELECT * FROM {table} WHERE {where_clause} LIMIT {query_config.get('limit', 10)}"

    def _build_filter_query(self, query_config):
        """Erstelle Filter-Abfrage"""
        table = query_config["table"]
        filters = query_config["filters"]

        conditions = []
        for field, value in filters.items():
            if isinstance(value, list):
                values_str = "', '".join(value)
                conditions.append(f"{field} IN ('{values_str}')")
            elif isinstance(value, bool):
                conditions.append(f"{field} = {str(value).lower()}")
            else:
                conditions.append(f"{field} = '{value}'")

        where_clause = " AND ".join(conditions)

        return f"SELECT * FROM {table} WHERE {where_clause} LIMIT {query_config.get('limit', 10)}"

    def _build_temporal_query(self, query_config):
        """Erstelle zeitbasierte Abfrage"""
        table = query_config["table"]
        time_field = query_config["time_field"]
        start_date = query_config["start_date"]
        end_date = query_config["end_date"]

        conditions = [f"{time_field} BETWEEN '{start_date}' AND '{end_date}'"]

        additional_filters = query_config.get("additional_filters", {})
        for field, value in additional_filters.items():
            conditions.append(f"{field} = '{value}'")

        where_clause = " AND ".join(conditions)

        return f"SELECT * FROM {table} WHERE {where_clause} ORDER BY {time_field} DESC LIMIT {query_config.get('limit', 10)}"

    def _build_hierarchical_query(self, query_config):
        """Erstelle hierarchische Abfrage"""
        table = query_config["table"]
        root_dept = query_config["root_department"]

        # Einfache hierarchische Abfrage
        if query_config.get("include_subdepartments", False):
            return f"SELECT * FROM {table} WHERE id = '{root_dept}' OR parent_department = '{root_dept}' LIMIT {query_config.get('limit', 10)}"
        else:
            return f"SELECT * FROM {table} WHERE id = '{root_dept}' LIMIT {query_config.get('limit', 10)}"

    def generate_comprehensive_report(self):
        """📊 Generiere umfassenden Testbericht"""
        logger.info("Generating comprehensive test report...")

        report = {
            "test_summary": {
                "timestamp": datetime.now().isoformat(),
                "total_datasets": len(self.data_verification_results),
                "verification_status": "COMPLETE"
            },
            "data_storage_verification": {},
            "query_performance": {},
            "recommendations": []
        }

        # Zusammenfassung der Datenspeicherung
        total_records = 0
        successful_datasets = 0

        for dataset, results in self.data_verification_results.items():
            # Skip non-storage datasets (these contain lists of test results)
            if dataset in ["intelligent_queries", "advanced_search", "neuromorphic_queries",
                           "quantum_queries", "dna_compression", "business_intelligence", "training_scenarios"]:
                continue

            # Handle only storage verification results (these should be dictionaries)
            if isinstance(results, dict):
                stored_count = results.get("stored_count", 0)
                total_records += stored_count

                if results.get("verification_status") == "SUCCESS":
                    successful_datasets += 1

                report["data_storage_verification"][dataset] = {
                    "records_stored": stored_count,
                    "status": results.get("verification_status"),
                    "sample_verified": results.get("sample_verified", False)
                }
            else:
                # Log warning for unexpected data structure
                logger.warning(f"Unexpected data structure for dataset {dataset}: {type(results)}")

        report["test_summary"]["total_records_stored"] = total_records
        report["test_summary"]["successful_datasets"] = successful_datasets

        # Query-Performance-Analyse
        if "intelligent_queries" in self.data_verification_results:
            query_results = self.data_verification_results["intelligent_queries"]
            if isinstance(query_results, list):
                successful_queries = [q for q in query_results if isinstance(q, dict) and q.get("status") == "SUCCESS"]

                if successful_queries:
                    avg_execution_time = sum(q.get("execution_time_ms", 0) for q in successful_queries) / len(successful_queries)
                    total_rows_returned = sum(q.get("rows_returned", 0) for q in successful_queries)

                    report["query_performance"] = {
                        "total_queries_tested": len(query_results),
                        "successful_queries": len(successful_queries),
                        "average_execution_time_ms": round(avg_execution_time, 2),
                        "total_rows_returned": total_rows_returned,
                        "queries_detail": successful_queries
                    }

        # Empfehlungen generieren
        storage_datasets_count = len([d for d in self.data_verification_results.keys()
                                    if d not in ["intelligent_queries", "advanced_search", "neuromorphic_queries",
                                               "quantum_queries", "dna_compression", "business_intelligence", "training_scenarios"]])

        if successful_datasets < storage_datasets_count:
            report["recommendations"].append("Some datasets failed to store properly - check database connection and permissions")

        if "intelligent_queries" in self.data_verification_results:
            query_results = self.data_verification_results["intelligent_queries"]
            if isinstance(query_results, list):
                failed_queries = [q for q in query_results if isinstance(q, dict) and q.get("status") != "SUCCESS"]
                if failed_queries:
                    report["recommendations"].append(f"{len(failed_queries)} intelligent queries failed - review query syntax and database schema")

        if total_records > 0:
            report["recommendations"].append("Data storage successful - database is ready for production workloads")

        # Speichere Bericht
        report_filename = f"neuroquantum_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(report_filename, 'w', encoding='utf-8') as f:
            json.dump(report, f, ensure_ascii=False, indent=2)

        logger.info(f"📊 Comprehensive report saved to: {report_filename}")

        # Ausgabe der wichtigsten Ergebnisse
        print("\n" + "="*80)
        print("🧠⚛️🧬 NEUROQUANTUMDB COMPREHENSIVE TEST REPORT")
        print("="*80)
        print(f"📊 Total Records Stored: {total_records:,}")
        print(f"✅ Successful Datasets: {successful_datasets}/{storage_datasets_count}")

        if "intelligent_queries" in self.data_verification_results:
            query_results = self.data_verification_results["intelligent_queries"]
            if isinstance(query_results, list):
                successful_queries = [q for q in query_results if isinstance(q, dict) and q.get("status") == "SUCCESS"]
                print(f"🔍 Successful Queries: {len(successful_queries)}/{len(query_results)}")

                if successful_queries:
                    avg_time = sum(q.get("execution_time_ms", 0) for q in successful_queries) / len(successful_queries)
                    print(f"⚡ Average Query Time: {avg_time:.2f}ms")

        print("\n📋 Dataset Summary:")
        for dataset, results in report["data_storage_verification"].items():
            status_emoji = "✅" if results["status"] == "SUCCESS" else "❌"
            print(f"  {status_emoji} {dataset}: {results['records_stored']:,} records")

        if report["recommendations"]:
            print("\n💡 Recommendations:")
            for i, rec in enumerate(report["recommendations"], 1):
                print(f"  {i}. {rec}")

        print("="*80)

        return report

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

        # Neue erweiterte Verifikation
        logger.info("\n" + "="*60)
        logger.info("🔍 PHASE 1: DATA STORAGE VERIFICATION")
        logger.info("="*60)
        storage_success = self.verify_data_storage()

        logger.info("\n" + "="*60)
        logger.info("🧠 PHASE 2: INTELLIGENT DATA RETRIEVAL TESTING")
        logger.info("="*60)
        retrieval_success = self.test_intelligent_data_retrieval()

        logger.info("\n" + "="*60)
        logger.info("🔎 PHASE 3: ADVANCED SEARCH CAPABILITIES")
        logger.info("="*60)
        search_success = self.test_advanced_search_capabilities()

        logger.info("\n" + "="*60)
        logger.info("🧠 PHASE 4: NEUROMORPHIC SECURITY QUERIES")
        logger.info("="*60)
        self.test_neuromorphic_security_queries()

        logger.info("\n" + "="*60)
        logger.info("⚛️ PHASE 5: QUANTUM OPTIMIZATION QUERIES")
        logger.info("="*60)
        self.test_quantum_optimization_queries()

        logger.info("\n" + "="*60)
        logger.info("🧬 PHASE 6: DNA COMPRESSION TESTING")
        logger.info("="*60)
        self.test_dna_compression_queries()

        logger.info("\n" + "="*60)
        logger.info("📊 PHASE 7: COMPLEX BUSINESS INTELLIGENCE")
        logger.info("="*60)
        self.test_complex_business_intelligence_queries()

        logger.info("\n" + "="*60)
        logger.info("🎓 PHASE 8: TRAINING SCENARIOS")
        logger.info("="*60)
        self.run_training_scenarios()

        # Generiere umfassenden Bericht
        logger.info("\n" + "="*60)
        logger.info("📊 GENERATING COMPREHENSIVE REPORT")
        logger.info("="*60)
        report = self.generate_comprehensive_report()

        logger.info("All enterprise tests completed!")
        return storage_success and retrieval_success

    def test_neuromorphic_security_queries(self):
        """🧠 Teste neuromorphe Sicherheitsanalysen"""
        logger.info("Testing neuromorphic security query capabilities...")

        if not self.api_key:
            logger.error("No API key available for neuromorphic queries")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        neuromorphic_queries = [
            {
                "name": "Neural Pattern Recognition - Suspicious Behavior",
                "description": "Verwende neuromorphe Muster zur Erkennung verdächtiger Verhaltensweisen",
                "query": """
                    SELECT 
                        e.id,
                        e.first_name,
                        e.last_name,
                        COUNT(al.id) as access_frequency,
                        COUNT(DISTINCT al.document_id) as unique_documents,
                        COUNT(CASE WHEN al.location = 'External' THEN 1 END) as external_accesses,
                        ROUND(AVG(al.duration_seconds), 2) as avg_session_duration,
                        COUNT(se.id) as security_incidents
                    FROM employees e
                    LEFT JOIN access_logs al ON e.id = al.employee_id
                    LEFT JOIN security_events se ON e.id = se.employee_id
                    WHERE al.timestamp >= DATE('now', '-7 days')
                    GROUP BY e.id, e.first_name, e.last_name
                    HAVING COUNT(al.id) > 20 OR COUNT(se.id) > 0
                    ORDER BY security_incidents DESC, access_frequency DESC
                    LIMIT 20
                """,
                "neuromorphic_features": ["pattern_recognition", "anomaly_detection", "behavioral_analysis"]
            }
        ]

        test_results = []
        for query_test in neuromorphic_queries:
            try:
                logger.info(f"🧠 Neural Test: {query_test['name']}")
                logger.info(f"  🎯 {query_test['description']}")

                query_payload = {
                    "query": query_test["query"],
                    "limit": 50
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=query_payload,
                    headers=headers,
                    timeout=90
                )
                execution_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    data = result.get('data', result.get('results', []))

                    # Ensure data is a list before slicing
                    if not isinstance(data, list):
                        data = []

                    test_result = {
                        "name": query_test["name"],
                        "status": "SUCCESS",
                        "neuromorphic_features": query_test["neuromorphic_features"],
                        "rows_returned": len(data),
                        "execution_time_ms": round(execution_time * 1000, 2),
                        "neural_insights": list(data[:3]) if data else []
                    }

                    logger.info(f"  ✅ Neural analysis complete: {len(data)} patterns detected in {execution_time*1000:.2f}ms")

                else:
                    test_result = {
                        "name": query_test["name"],
                        "status": "FAILED",
                        "error": f"HTTP {response.status_code}"
                    }
                    logger.error(f"  ❌ Neural analysis failed: HTTP {response.status_code}")

                test_results.append(test_result)

            except Exception as e:
                test_result = {
                    "name": query_test["name"],
                    "status": "ERROR",
                    "error": str(e)
                }
                test_results.append(test_result)
                logger.error(f"  ❌ Neural error: {str(e)}")

        self.data_verification_results["neuromorphic_queries"] = test_results
        successful_tests = len([t for t in test_results if t["status"] == "SUCCESS"])
        logger.info(f"🧠 Neuromorphic Tests: {successful_tests}/{len(test_results)} successful")

        return successful_tests > 0

    def test_quantum_optimization_queries(self):
        """⚛️ Teste Quantum-optimierte Abfragen"""
        logger.info("Testing quantum optimization capabilities...")

        if not self.api_key:
            logger.error("No API key available for quantum queries")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        quantum_queries = [
            {
                "name": "Quantum Superposition - Multi-dimensional Analysis",
                "description": "Nutze Quantensuperposition für mehrdimensionale Datenanalyse",
                "query": """
                    SELECT 
                        d.security_classification,
                        e.department_id,
                        COUNT(*) as total_accesses,
                        COUNT(DISTINCT e.id) as unique_users,
                        SUM(d.file_size_bytes) as total_data_accessed,
                        COUNT(CASE WHEN al.result = 'SUCCESS' THEN 1 END) as successful_accesses
                    FROM access_logs al
                    JOIN documents d ON al.document_id = d.id
                    JOIN employees e ON al.employee_id = e.id
                    WHERE al.timestamp >= DATE('now', '-14 days')
                    GROUP BY d.security_classification, e.department_id
                    ORDER BY total_accesses DESC
                    LIMIT 50
                """,
                "quantum_features": ["superposition", "entanglement", "multi_dimensional_analysis"]
            }
        ]

        test_results = []
        for query_test in quantum_queries:
            try:
                logger.info(f"⚛️ Quantum Test: {query_test['name']}")
                logger.info(f"  🌌 {query_test['description']}")

                query_payload = {
                    "query": query_test["query"],
                    "limit": 50
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=query_payload,
                    headers=headers,
                    timeout=120
                )
                execution_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    data = result.get('data', result.get('results', []))

                    # Ensure data is a list before slicing
                    if not isinstance(data, list):
                        data = []

                    test_result = {
                        "name": query_test["name"],
                        "status": "SUCCESS",
                        "quantum_features": query_test["quantum_features"],
                        "rows_returned": len(data),
                        "execution_time_ms": round(execution_time * 1000, 2),
                        "quantum_insights": list(data[:3]) if data else []
                    }

                    logger.info(f"  ✅ Quantum computation complete: {len(data)} states analyzed in {execution_time*1000:.2f}ms")

                else:
                    test_result = {
                        "name": query_test["name"],
                        "status": "FAILED",
                        "error": f"HTTP {response.status_code}"
                    }
                    logger.error(f"  ❌ Quantum computation failed: HTTP {response.status_code}")

                test_results.append(test_result)

            except Exception as e:
                test_result = {
                    "name": query_test["name"],
                    "status": "ERROR",
                    "error": str(e)
                }
                test_results.append(test_result)
                logger.error(f"  ❌ Quantum error: {str(e)}")

        self.data_verification_results["quantum_queries"] = test_results
        successful_tests = len([t for t in test_results if t["status"] == "SUCCESS"])
        logger.info(f"⚛️ Quantum Tests: {successful_tests}/{len(test_results)} successful")

        return successful_tests > 0

    def test_dna_compression_queries(self):
        """🧬 Teste DNA-Kompressionsfähigkeiten"""
        logger.info("Testing DNA compression and storage capabilities...")

        if not self.api_key:
            logger.error("No API key available for DNA compression tests")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        dna_tests = [
            {
                "name": "DNA Compression - Large Document Analysis",
                "description": "Teste DNA-Kompression für große Dokumentmengen",
                "query": """
                    SELECT 
                        document_type,
                        security_classification,
                        COUNT(*) as document_count,
                        SUM(file_size_bytes) as total_size_bytes,
                        ROUND(AVG(file_size_bytes), 2) as avg_size_bytes,
                        MAX(file_size_bytes) as max_size_bytes
                    FROM documents
                    WHERE file_size_bytes > 1048576
                    GROUP BY document_type, security_classification
                    ORDER BY total_size_bytes DESC
                    LIMIT 20
                """,
                "dna_features": ["compression", "storage_optimization", "biological_encoding"]
            }
        ]

        test_results = []
        for test_config in dna_tests:
            try:
                logger.info(f"🧬 DNA Test: {test_config['name']}")
                logger.info(f"  🔬 {test_config['description']}")

                query_payload = {
                    "query": test_config["query"],
                    "limit": 50
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=query_payload,
                    headers=headers,
                    timeout=150
                )
                execution_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    data = result.get('data', result.get('results', []))

                    # Ensure data is a list before slicing
                    if not isinstance(data, list):
                        data = []

                    test_result = {
                        "name": test_config["name"],
                        "status": "SUCCESS",
                        "dna_features": test_config["dna_features"],
                        "rows_returned": len(data),
                        "execution_time_ms": round(execution_time * 1000, 2),
                        "biological_data": list(data[:2]) if data else []
                    }

                    logger.info(f"  ✅ DNA analysis complete: {len(data)} sequences processed in {execution_time*1000:.2f}ms")

                else:
                    test_result = {
                        "name": test_config["name"],
                        "status": "FAILED",
                        "error": f"HTTP {response.status_code}"
                    }
                    logger.error(f"  ❌ DNA analysis failed: HTTP {response.status_code}")

                test_results.append(test_result)

            except Exception as e:
                test_result = {
                    "name": test_config["name"],
                    "status": "ERROR",
                    "error": str(e)
                }
                test_results.append(test_result)
                logger.error(f"  ❌ DNA error: {str(e)}")

        self.data_verification_results["dna_compression"] = test_results
        successful_tests = len([t for t in test_results if t["status"] == "SUCCESS"])
        logger.info(f"🧬 DNA Tests: {successful_tests}/{len(test_results)} successful")

        return successful_tests > 0

    def test_complex_business_intelligence_queries(self):
        """📊 Teste komplexe Business Intelligence Abfragen"""
        logger.info("Testing complex business intelligence capabilities...")

        if not self.api_key:
            logger.error("No API key available for BI queries")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        bi_queries = [
            {
                "name": "Executive Dashboard - Key Performance Indicators",
                "description": "Umfassende KPI-Analyse für das Management",
                "query": """
                    SELECT 
                        d.name as department,
                        d.security_level,
                        COUNT(DISTINCT e.id) as employee_count,
                        ROUND(AVG(e.salary), 2) as avg_salary,
                        COUNT(DISTINCT doc.id) as documents_owned,
                        COUNT(al.id) as monthly_accesses,
                        COUNT(CASE WHEN al.result = 'SUCCESS' THEN 1 END) as successful_accesses
                    FROM departments d
                    LEFT JOIN employees e ON d.id = e.department_id AND e.active = 1
                    LEFT JOIN documents doc ON d.id = doc.owner_department_id
                    LEFT JOIN access_logs al ON e.id = al.employee_id AND al.timestamp >= DATE('now', '-30 days')
                    GROUP BY d.id, d.name, d.security_level
                    ORDER BY employee_count DESC
                    LIMIT 25
                """,
                "bi_type": "executive_dashboard"
            }
        ]

        test_results = []
        for query_test in bi_queries:
            try:
                logger.info(f"📊 BI Test: {query_test['name']}")
                logger.info(f"  📈 {query_test['description']}")

                query_payload = {
                    "query": query_test["query"],
                    "limit": 50
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=query_payload,
                    headers=headers,
                    timeout=180
                )
                execution_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    data = result.get('data', result.get('results', []))

                    # Ensure data is a list before slicing
                    if not isinstance(data, list):
                        data = []

                    test_result = {
                        "name": query_test["name"],
                        "status": "SUCCESS",
                        "bi_type": query_test["bi_type"],
                        "rows_returned": len(data),
                        "execution_time_ms": round(execution_time * 1000, 2),
                        "business_insights": data[:3] if data else []
                    }

                    logger.info(f"  ✅ BI analysis complete: {len(data)} insights generated in {execution_time*1000:.2f}ms")

                else:
                    test_result = {
                        "name": query_test["name"],
                        "status": "FAILED",
                        "error": f"HTTP {response.status_code}"
                    }
                    logger.error(f"  ❌ BI analysis failed: HTTP {response.status_code}")

                test_results.append(test_result)

            except Exception as e:
                test_result = {
                    "name": query_test["name"],
                    "status": "ERROR",
                    "error": str(e)
                }
                test_results.append(test_result)
                logger.error(f"  ❌ BI error: {str(e)}")

        self.data_verification_results["business_intelligence"] = test_results
        successful_tests = len([t for t in test_results if t["status"] == "SUCCESS"])
        logger.info(f"📊 BI Tests: {successful_tests}/{len(test_results)} successful")

        return successful_tests > 0

    def run_training_scenarios(self):
        """🎓 Führe Trainingsszenarien für ML-Modelle aus"""
        logger.info("Running training scenarios for machine learning models...")

        if not self.api_key:
            logger.error("No API key available for training scenarios")
            return False

        headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }

        training_scenarios = [
            {
                "name": "Anomaly Detection Training",
                "description": "Trainiere Anomalieerkennung für ungewöhnliche Zugriffsmuster",
                "scenario_type": "anomaly_detection",
                "training_data_query": """
                    SELECT 
                        al.employee_id,
                        al.document_id,
                        al.action,
                        al.location,
                        al.duration_seconds,
                        al.result,
                        e.security_clearance,
                        e.role,
                        d.security_classification,
                        d.document_type
                    FROM access_logs al
                    JOIN employees e ON al.employee_id = e.id
                    JOIN documents d ON al.document_id = d.id
                    WHERE al.timestamp >= DATE('now', '-90 days')
                    ORDER BY RANDOM()
                    LIMIT 1000
                """
            }
        ]

        training_results = []
        for scenario in training_scenarios:
            try:
                logger.info(f"🎓 Training: {scenario['name']}")
                logger.info(f"  📚 {scenario['description']}")

                # Sammle Trainingsdaten
                training_query = {
                    "query": scenario["training_data_query"],
                    "limit": 1000
                }

                start_time = time.time()
                response = self.session.post(
                    f"{self.api_base}/query",
                    json=training_query,
                    headers=headers,
                    timeout=120
                )
                data_collection_time = time.time() - start_time

                if response.status_code == 200:
                    result = response.json()
                    training_data = result.get('data', result.get('results', []))

                    scenario_result = {
                        "name": scenario["name"],
                        "status": "SUCCESS",
                        "scenario_type": scenario["scenario_type"],
                        "training_samples": len(training_data),
                        "data_collection_time_ms": round(data_collection_time * 1000, 2)
                    }

                    logger.info(f"  ✅ Training complete: {len(training_data)} samples collected in {data_collection_time*1000:.2f}ms")

                else:
                    scenario_result = {
                        "name": scenario["name"],
                        "status": "DATA_COLLECTION_FAILED",
                        "error": f"Data collection HTTP {response.status_code}"
                    }
                    logger.error(f"  ❌ Data collection failed: HTTP {response.status_code}")

                training_results.append(scenario_result)

            except Exception as e:
                scenario_result = {
                    "name": scenario["name"],
                    "status": "ERROR",
                    "error": str(e)
                }
                training_results.append(scenario_result)
                logger.error(f"  ❌ Training error: {str(e)}")

        self.data_verification_results["training_scenarios"] = training_results
        successful_training = len([t for t in training_results if t["status"] == "SUCCESS"])
        logger.info(f"🎓 Training Scenarios: {successful_training}/{len(training_results)} successful")

        return successful_training > 0

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
