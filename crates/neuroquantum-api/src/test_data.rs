//! Test Data Module
//! Contains mock data for testing purposes only
//! This module should only be compiled in test builds

#[cfg(test)]
use serde_json::json;

#[cfg(test)]
pub fn get_mock_employees() -> Vec<serde_json::Value> {
    vec![
        json!({
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
            "active": true
        }),
        json!({
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
            "active": true
        })
    ]
}

#[cfg(test)]
pub fn get_mock_departments() -> Vec<serde_json::Value> {
    vec![
        json!({
            "id": "DEPT_001",
            "name": "IT_Abteilung",
            "description": "Informationstechnologie",
            "security_level": "VERTRAULICH",
            "budget": 500000,
            "employee_count": 25,
            "location": "Berlin"
        }),
        json!({
            "id": "DEPT_002",
            "name": "Personal_HR",
            "description": "Human Resources",
            "security_level": "GEHEIM",
            "budget": 300000,
            "employee_count": 15,
            "location": "München"
        })
    ]
}
