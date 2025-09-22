//! Realistische Testdaten für NeuroQuantumDB
//!
//! Diese Datei enthält realistische Datensätze, die die volle Komplexität
//! der Datenbank demonstrieren und echte Anwendungsszenarien abbilden.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// IoT Sensor-Daten für Edge Computing Szenarien
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTSensorData {
    pub sensor_id: Uuid,
    pub device_type: String,
    pub location: GeoLocation,
    pub timestamp: DateTime<Utc>,
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub air_quality: AirQuality,
    pub battery_level: u8,
    pub signal_strength: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirQuality {
    pub pm25: f32,
    pub pm10: f32,
    pub co2: u16,
    pub no2: f32,
    pub ozone: f32,
}

/// Medizinische Patientendaten für neuromorphe Analyse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientData {
    pub patient_id: Uuid,
    pub age: u8,
    pub gender: Gender,
    pub vital_signs: VitalSigns,
    pub symptoms: Vec<String>,
    pub medical_history: Vec<MedicalEvent>,
    pub genomic_markers: Vec<GenomicMarker>,
    pub brain_activity: BrainActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalSigns {
    pub heart_rate: u16,
    pub blood_pressure_systolic: u16,
    pub blood_pressure_diastolic: u16,
    pub body_temperature: f32,
    pub oxygen_saturation: f32,
    pub respiratory_rate: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalEvent {
    pub date: DateTime<Utc>,
    pub event_type: String,
    pub severity: u8, // 1-10
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomicMarker {
    pub gene_id: String,
    pub variant: String,
    pub risk_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainActivity {
    pub eeg_data: Vec<f32>, // EEG Werte
    pub neural_patterns: Vec<NeuralPattern>,
    pub cognitive_load: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralPattern {
    pub frequency_band: String, // Alpha, Beta, Gamma, etc.
    pub amplitude: f32,
    pub coherence: f32,
}

/// Finanzmarkt-Daten für Quantum Trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialData {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub volume: u64,
    pub market_data: MarketData,
    pub sentiment_analysis: SentimentData,
    pub quantum_indicators: QuantumIndicators,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub vwap: f64, // Volume Weighted Average Price
    pub volatility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentData {
    pub news_sentiment: f32, // -1.0 bis 1.0
    pub social_sentiment: f32,
    pub analyst_rating: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumIndicators {
    pub quantum_momentum: f32,
    pub entanglement_strength: f32,
    pub superposition_state: Vec<f32>,
}

/// Factory für realistische Testdaten
pub struct TestDataFactory;

impl TestDataFactory {
    /// Generiert IoT Sensor-Daten für verschiedene Standorte
    pub fn generate_iot_data(count: usize) -> Vec<IoTSensorData> {
        let locations = [
            ("Berlin", 52.5200, 13.4050),
            ("Hamburg", 53.5511, 9.9937),
            ("München", 48.1351, 11.5820),
            ("Köln", 50.9375, 6.9603),
            ("Frankfurt", 50.1109, 8.6821),
        ];

        (0..count)
            .map(|i| {
                let location = &locations[i % locations.len()];
                IoTSensorData {
                    sensor_id: Uuid::new_v4(),
                    device_type: format!("ESP32-{}", i % 5),
                    location: GeoLocation {
                        latitude: location.1 + (rand::random::<f64>() - 0.5) * 0.1,
                        longitude: location.2 + (rand::random::<f64>() - 0.5) * 0.1,
                        altitude: Some(rand::random::<f32>() * 200.0),
                    },
                    timestamp: Utc::now() - chrono::Duration::minutes(rand::random::<i64>() % 1440),
                    temperature: 15.0 + rand::random::<f32>() * 25.0,
                    humidity: 30.0 + rand::random::<f32>() * 40.0,
                    pressure: 980.0 + rand::random::<f32>() * 60.0,
                    air_quality: AirQuality {
                        pm25: rand::random::<f32>() * 50.0,
                        pm10: rand::random::<f32>() * 100.0,
                        co2: (350 + rand::random::<u16>() % 1000),
                        no2: rand::random::<f32>() * 0.2,
                        ozone: rand::random::<f32>() * 0.15,
                    },
                    battery_level: rand::random::<u8>() % 101,
                    signal_strength: -40 - (rand::random::<i8>() % 50),
                }
            })
            .collect()
    }

    /// Generiert medizinische Patientendaten
    pub fn generate_patient_data(count: usize) -> Vec<PatientData> {
        (0..count)
            .map(|i| {
                PatientData {
                    patient_id: Uuid::new_v4(),
                    age: 18 + rand::random::<u8>() % 80,
                    gender: match i % 3 {
                        0 => Gender::Male,
                        1 => Gender::Female,
                        _ => Gender::Other,
                    },
                    vital_signs: VitalSigns {
                        heart_rate: 60 + rand::random::<u16>() % 80,
                        blood_pressure_systolic: 90 + rand::random::<u16>() % 60,
                        blood_pressure_diastolic: 60 + rand::random::<u16>() % 40,
                        body_temperature: 36.0 + rand::random::<f32>() * 3.0,
                        oxygen_saturation: 95.0 + rand::random::<f32>() * 5.0,
                        respiratory_rate: 12 + rand::random::<u16>() % 8,
                    },
                    symptoms: {
                        let symptom_pool = vec![
                            "Kopfschmerzen".to_string(),
                            "Müdigkeit".to_string(),
                            "Schwindel".to_string(),
                        ];
                        let num_symptoms = 1 + rand::random::<usize>() % 3; // Mindestens 1 Symptom
                        symptom_pool.into_iter().take(num_symptoms).collect()
                    },
                    medical_history: vec![MedicalEvent {
                        date: Utc::now() - chrono::Duration::days(rand::random::<i64>() % 3650),
                        event_type: "Routine Checkup".to_string(),
                        severity: 1 + rand::random::<u8>() % 3,
                        description: "Jährliche Untersuchung".to_string(),
                    }],
                    genomic_markers: vec![GenomicMarker {
                        gene_id: "APOE".to_string(),
                        variant: "ε3/ε3".to_string(),
                        risk_score: rand::random::<f32>(),
                    }],
                    brain_activity: BrainActivity {
                        eeg_data: (0..256).map(|_| rand::random::<f32>() * 100.0).collect(),
                        neural_patterns: vec![NeuralPattern {
                            frequency_band: "Alpha".to_string(),
                            amplitude: rand::random::<f32>() * 50.0,
                            coherence: rand::random::<f32>(),
                        }],
                        cognitive_load: rand::random::<f32>(),
                    },
                }
            })
            .collect()
    }

    /// Generiert Finanzmarkt-Daten
    pub fn generate_financial_data(count: usize) -> Vec<FinancialData> {
        let symbols = ["AAPL", "GOOGL", "MSFT", "TSLA", "AMZN", "META", "NVDA"];

        (0..count)
            .map(|i| {
                let base_price = 100.0 + rand::random::<f64>() * 400.0;
                FinancialData {
                    symbol: symbols[i % symbols.len()].to_string(),
                    timestamp: Utc::now() - chrono::Duration::minutes(rand::random::<i64>() % 1440),
                    price: base_price,
                    volume: rand::random::<u64>() % 10_000_000,
                    market_data: MarketData {
                        open: base_price * (0.95 + rand::random::<f64>() * 0.1),
                        high: base_price * (1.0 + rand::random::<f64>() * 0.05),
                        low: base_price * (0.95 + rand::random::<f64>() * 0.05),
                        close: base_price,
                        vwap: base_price * (0.98 + rand::random::<f64>() * 0.04),
                        volatility: rand::random::<f32>() * 0.5,
                    },
                    sentiment_analysis: SentimentData {
                        news_sentiment: -1.0 + rand::random::<f32>() * 2.0,
                        social_sentiment: -1.0 + rand::random::<f32>() * 2.0,
                        analyst_rating: -1.0 + rand::random::<f32>() * 2.0,
                        confidence: rand::random::<f32>(),
                    },
                    quantum_indicators: QuantumIndicators {
                        quantum_momentum: rand::random::<f32>() * 2.0 - 1.0,
                        entanglement_strength: rand::random::<f32>(),
                        superposition_state: (0..8).map(|_| rand::random::<f32>()).collect(),
                    },
                }
            })
            .collect()
    }

    /// Generiert komplexe QSQL Testqueries
    pub fn get_test_queries() -> Vec<&'static str> {
        vec![
            // Basis SQL-Kompatibilität
            "SELECT * FROM sensors WHERE temperature > 25.0",
            // Neuromorphic Extensions
            "SELECT * FROM patients NEUROMATCH symptoms LIKE '%Kopfschmerzen%'
             WITH PLASTICITY 0.8 SYNAPTIC_THRESHOLD 0.6",
            // Quantum-inspired Joins
            "SELECT s.sensor_id, p.patient_id
             FROM sensors s QUANTUM_JOIN patients p
             ON SUPERPOSITION(s.location, p.location)
             WHERE ENTANGLEMENT_STRENGTH > 0.7",
            // DNA-based Compression Query
            "SELECT COMPRESS_DNA(symptoms) as compressed_symptoms
             FROM patients
             WHERE age BETWEEN 30 AND 50
             GROUP BY genomic_markers",
            // Natural Language Query
            "FIND all sensors in Berlin with high temperature and low battery",
            // Complex Neuromorphic Learning
            "LEARN PATTERN brain_activity
             FROM patients
             WHERE symptoms CONTAINS 'Schwindel'
             ADAPT SYNAPTIC_WEIGHTS WITH HEBBIAN_RULE
             STORE IN neural_patterns_cache",
            // Quantum Search with Grover's Algorithm
            "QUANTUM_SEARCH financial_data
             WHERE symbol IN ('AAPL', 'GOOGL')
             AND quantum_momentum > 0.5
             AMPLIFY WITH GROVERS_ALGORITHM
             MAX_ITERATIONS 10",
        ]
    }
}

/// Erwartete Ergebnisse für Validierung
pub struct ExpectedResults;

impl ExpectedResults {
    pub fn quantum_search_speedup() -> f32 {
        // Erwartete quadratische Beschleunigung bei Quantum Search
        2.0
    }

    pub fn dna_compression_ratio() -> f32 {
        // Erwartete DNA-Kompressionsrate
        4.0
    }

    pub fn neuromorphic_learning_accuracy() -> f32 {
        // Erwartete Lerngenauigkeit
        0.85
    }

    pub fn api_response_time_ms() -> u64 {
        // Maximale akzeptable API-Antwortzeit
        500
    }
}
