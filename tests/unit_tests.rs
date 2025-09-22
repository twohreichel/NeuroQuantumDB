//! Unit Tests für NeuroQuantumDB Kernmodule
//!
//! Diese Tests fokussieren sich auf einzelne Komponenten und deren
//! spezifische Funktionalitäten mit deterministischen Ergebnissen.

use std::collections::HashMap;
use tokio::test;
use anyhow::Result;

// Import der Testdaten
mod test_data;
use test_data::*;

/// Tests für DNA-basierte Datenkompression
mod dna_compression_tests {
    use super::*;
    use neuroquantum_core::dna::{DNACompressor, DNABase, DNASequence};

    #[test]
    fn test_dna_base_encoding() {
        // Teste DNA Base zu Bit Konvertierung
        assert_eq!(DNABase::Adenine as u8, 0b00);
        assert_eq!(DNABase::Thymine as u8, 0b01);
        assert_eq!(DNABase::Guanine as u8, 0b10);
        assert_eq!(DNABase::Cytosine as u8, 0b11);

        println!("✅ DNA base encoding works correctly");
    }

    #[test]
    fn test_text_to_dna_compression() -> Result<()> {
        let compressor = DNACompressor::new();

        // Teste verschiedene Datentypen
        let test_cases = vec![
            ("Hello World", "ATCGATCGTAGC"), // Vereinfachtes Beispiel
            ("NeuroQuantumDB", "GCTAGCATCGAT"),
            ("", ""), // Edge case: leerer String
        ];

        for (input, expected_pattern) in test_cases {
            let compressed = compressor.compress_text(input)?;
            assert!(!compressed.is_empty() || input.is_empty());

            // Teste Dekompression
            let decompressed = compressor.decompress_to_text(&compressed)?;
            assert_eq!(input, decompressed);
        }

        println!("✅ Text to DNA compression works correctly");
        Ok(())
    }

    #[test]
    fn test_binary_data_compression() -> Result<()> {
        let compressor = DNACompressor::new();

        // Teste IoT Sensordaten
        let sensor_data = TestDataFactory::generate_iot_data(1)[0].clone();
        let binary_data = serde_json::to_vec(&sensor_data)?;

        let compressed = compressor.compress_binary(&binary_data)?;
        let decompressed = compressor.decompress_binary(&compressed)?;

        assert_eq!(binary_data, decompressed);

        // Teste Kompressionsrate
        let compression_ratio = binary_data.len() as f32 / compressed.len() as f32;
        assert!(compression_ratio >= 2.0, "Compression ratio too low: {}", compression_ratio);

        println!("✅ Binary data DNA compression works correctly (ratio: {:.2})", compression_ratio);
        Ok(())
    }

    #[test]
    fn test_dna_sequence_operations() -> Result<()> {
        let mut sequence = DNASequence::new();

        // Teste Sequenz-Operationen
        sequence.append_base(DNABase::Adenine);
        sequence.append_base(DNABase::Thymine);
        sequence.append_base(DNABase::Guanine);
        sequence.append_base(DNABase::Cytosine);

        assert_eq!(sequence.length(), 4);
        assert_eq!(sequence.get_base(0)?, DNABase::Adenine);
        assert_eq!(sequence.to_string(), "ATGC");

        // Teste Reverse Complement
        let complement = sequence.reverse_complement();
        assert_eq!(complement.to_string(), "GCAT");

        println!("✅ DNA sequence operations work correctly");
        Ok(())
    }
}

/// Tests für Quantum Computing Module
mod quantum_tests {
    use super::*;
    use neuroquantum_core::quantum::{QuantumProcessor, QuantumState, QuantumGate};

    #[test]
    fn test_quantum_superposition() -> Result<()> {
        let mut processor = QuantumProcessor::new(3); // 3 Qubits

        // Initialisiere in Superposition
        processor.apply_hadamard(0)?;
        processor.apply_hadamard(1)?;
        processor.apply_hadamard(2)?;

        let state = processor.get_state();

        // Nach Hadamard-Gates sollten alle Amplituden gleich sein
        let expected_amplitude = 1.0 / (8.0_f64).sqrt(); // 2^3 = 8 Zustände
        for amplitude in state.amplitudes {
            assert!((amplitude.norm() - expected_amplitude).abs() < 1e-10);
        }

        println!("✅ Quantum superposition works correctly");
        Ok(())
    }

    #[test]
    fn test_grovers_algorithm_simulation() -> Result<()> {
        let processor = QuantumProcessor::new(4); // 4 Qubits = 16 Suchraum

        // Simuliere Grover's Algorithmus für Suche nach Element 10
        let target_item = 10;
        let search_space_size = 16;

        let result = processor.grovers_search(target_item, search_space_size)?;

        // Grover's sollte das gesuchte Element mit hoher Wahrscheinlichkeit finden
        assert_eq!(result.found_item, target_item);
        assert!(result.probability > 0.9, "Grover's probability too low: {}", result.probability);

        // Teste erwartete Anzahl von Iterationen
        let expected_iterations = ((std::f64::consts::PI / 4.0) * (search_space_size as f64).sqrt()).ceil() as usize;
        assert_eq!(result.iterations, expected_iterations);

        println!("✅ Grover's algorithm simulation works correctly");
        Ok(())
    }

    #[test]
    fn test_quantum_entanglement() -> Result<()> {
        let mut processor = QuantumProcessor::new(2);

        // Erstelle Bell-Zustand (maximale Verschränkung)
        processor.apply_hadamard(0)?;
        processor.apply_cnot(0, 1)?;

        let entanglement = processor.measure_entanglement(0, 1)?;

        // Bell-Zustand sollte maximale Verschränkung haben
        assert!(entanglement > 0.99, "Entanglement too low: {}", entanglement);

        println!("✅ Quantum entanglement works correctly");
        Ok(())
    }

    #[test]
    fn test_quantum_interference() -> Result<()> {
        let mut processor = QuantumProcessor::new(1);

        // Teste konstruktive und destruktive Interferenz
        processor.apply_hadamard(0)?;
        processor.apply_phase_gate(0, std::f64::consts::PI)?; // π-Phase
        processor.apply_hadamard(0)?;

        let state = processor.get_state();

        // Nach diesem Sequence sollte der Zustand |1⟩ sein (destruktive Interferenz für |0⟩)
        assert!(state.amplitudes[0].norm() < 1e-10); // |0⟩ Amplitude ≈ 0
        assert!((state.amplitudes[1].norm() - 1.0).abs() < 1e-10); // |1⟩ Amplitude ≈ 1

        println!("✅ Quantum interference works correctly");
        Ok(())
    }
}

/// Tests für Neuromorphic Learning
mod neuromorphic_tests {
    use super::*;
    use neuroquantum_core::synaptic::{SynapticNetwork, SynapticNode, NeuralPattern};
    use neuroquantum_core::plasticity::{HebbianRule, SynapticPlasticity};
    use neuroquantum_core::learning::{NeuromorphicLearner, LearningConfig};

    #[test]
    fn test_synaptic_network_creation() -> Result<()> {
        let network = SynapticNetwork::new(100, 50, 10); // 100 Input, 50 Hidden, 10 Output

        assert_eq!(network.input_layer_size(), 100);
        assert_eq!(network.hidden_layer_size(), 50);
        assert_eq!(network.output_layer_size(), 10);

        // Teste initiale Verbindungen
        let connectivity = network.get_connectivity_matrix();
        assert!(connectivity.density() > 0.1); // Mindestens 10% Verbindungen

        println!("✅ Synaptic network creation works correctly");
        Ok(())
    }

    #[test]
    fn test_hebbian_learning() -> Result<()> {
        let mut network = SynapticNetwork::new(10, 5, 2);
        let hebbian_rule = HebbianRule::new(0.01); // Lernrate 0.01

        // Trainiere mit einfachem Muster
        let input_pattern = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let target_output = vec![1.0, 0.0];

        let initial_weights = network.get_weights().clone();

        // Mehrere Trainingszyklen
        for _ in 0..100 {
            let output = network.forward_pass(&input_pattern)?;
            network.apply_hebbian_learning(&input_pattern, &output, &hebbian_rule)?;
        }

        let final_weights = network.get_weights();

        // Gewichte sollten sich verändert haben
        assert_ne!(initial_weights, *final_weights);

        // Teste ob Netzwerk das Muster gelernt hat
        let learned_output = network.forward_pass(&input_pattern)?;
        let error = calculate_mse(&learned_output, &target_output);
        assert!(error < 0.1, "Learning error too high: {}", error);

        println!("✅ Hebbian learning works correctly (error: {:.4})", error);
        Ok(())
    }

    #[test]
    fn test_synaptic_plasticity() -> Result<()> {
        let mut plasticity = SynapticPlasticity::new();

        // Teste Long-Term Potentiation (LTP)
        let initial_strength = 0.5;
        let stimulation_frequency = 100.0; // Hz
        let duration = 1.0; // Sekunde

        let new_strength = plasticity.apply_ltp(initial_strength, stimulation_frequency, duration)?;
        assert!(new_strength > initial_strength, "LTP should increase synaptic strength");

        // Teste Long-Term Depression (LTD)
        let ltd_strength = plasticity.apply_ltd(new_strength, 1.0, 10.0)?; // Niedrige Frequenz
        assert!(ltd_strength < new_strength, "LTD should decrease synaptic strength");

        println!("✅ Synaptic plasticity works correctly");
        Ok(())
    }

    #[test]
    fn test_pattern_recognition() -> Result<()> {
        let config = LearningConfig::default();
        let mut learner = NeuromorphicLearner::new(config);

        // Trainiere Muster-Erkennnung mit Patientendaten
        let patient_data = TestDataFactory::generate_patient_data(100);

        // Extrahiere Features (Symptome -> EEG Pattern)
        let training_data: Vec<(Vec<f32>, Vec<f32>)> = patient_data.iter().map(|p| {
            let symptoms_features = encode_symptoms(&p.symptoms);
            let eeg_features = p.brain_activity.eeg_data.clone();
            (symptoms_features, eeg_features)
        }).collect();

        // Training
        let training_result = learner.train_pattern_recognition(&training_data)?;
        assert!(training_result.accuracy > 0.7, "Pattern recognition accuracy too low");

        // Teste Vorhersage
        let test_patient = &patient_data[0];
        let test_input = encode_symptoms(&test_patient.symptoms);
        let prediction = learner.predict(&test_input)?;

        assert!(!prediction.is_empty(), "Prediction should not be empty");
        assert!(prediction.len() == test_patient.brain_activity.eeg_data.len());

        println!("✅ Pattern recognition works correctly (accuracy: {:.2})", training_result.accuracy);
        Ok(())
    }

    // Hilfsfunktionen
    fn calculate_mse(predicted: &[f32], actual: &[f32]) -> f32 {
        predicted.iter().zip(actual.iter())
            .map(|(p, a)| (p - a).powi(2))
            .sum::<f32>() / predicted.len() as f32
    }

    fn encode_symptoms(symptoms: &[String]) -> Vec<f32> {
        // Vereinfachte Symptom-Kodierung
        let symptom_map = HashMap::from([
            ("Kopfschmerzen".to_string(), 0),
            ("Müdigkeit".to_string(), 1),
            ("Schwindel".to_string(), 2),
        ]);

        let mut encoded = vec![0.0; 10]; // 10-dimensionaler Feature-Vektor
        for symptom in symptoms {
            if let Some(&index) = symptom_map.get(symptom) {
                if index < encoded.len() {
                    encoded[index] = 1.0;
                }
            }
        }
        encoded
    }
}

/// Tests für QSQL Parser und Optimizer
mod qsql_tests {
    use super::*;
    use neuroquantum_qsql::{parser::QSQLParser, ast::*, optimizer::NeuromorphicOptimizer};

    #[test]
    fn test_basic_sql_parsing() -> Result<()> {
        let parser = QSQLParser::new();

        let basic_queries = vec![
            "SELECT * FROM sensors",
            "SELECT temperature, humidity FROM sensors WHERE temperature > 25",
            "INSERT INTO sensors (id, temperature) VALUES ('sensor1', 23.5)",
            "UPDATE sensors SET temperature = 24.0 WHERE id = 'sensor1'",
            "DELETE FROM sensors WHERE battery_level < 10",
        ];

        for query in basic_queries {
            let ast = parser.parse(query)?;
            assert!(ast.is_valid(), "AST should be valid for: {}", query);
            println!("✅ Parsed: {}", query);
        }

        Ok(())
    }

    #[test]
    fn test_neuromorphic_extensions() -> Result<()> {
        let parser = QSQLParser::new();

        let neuro_queries = vec![
            "SELECT * FROM patients NEUROMATCH symptoms LIKE '%headache%'",
            "LEARN PATTERN brain_activity FROM patients",
            "ADAPT SYNAPTIC_WEIGHTS WITH HEBBIAN_RULE",
        ];

        for query in neuro_queries {
            let ast = parser.parse(query)?;
            assert!(ast.has_neuromorphic_features(), "Should have neuro features: {}", query);
            println!("✅ Parsed neuromorphic query: {}", query);
        }

        Ok(())
    }

    #[test]
    fn test_quantum_extensions() -> Result<()> {
        let parser = QSQLParser::new();

        let quantum_queries = vec![
            "QUANTUM_SEARCH financial_data WHERE symbol = 'AAPL'",
            "SELECT * FROM sensors s QUANTUM_JOIN locations l ON SUPERPOSITION(s.location, l.coords)",
            "AMPLIFY results WITH GROVERS_ALGORITHM MAX_ITERATIONS 10",
        ];

        for query in quantum_queries {
            let ast = parser.parse(query)?;
            assert!(ast.has_quantum_features(), "Should have quantum features: {}", query);
            println!("✅ Parsed quantum query: {}", query);
        }

        Ok(())
    }

    #[test]
    fn test_query_optimization() -> Result<()> {
        let optimizer = NeuromorphicOptimizer::new();

        let query = "SELECT * FROM sensors WHERE temperature > 25 AND battery_level < 20";
        let ast = QSQLParser::new().parse(query)?;

        let optimized_plan = optimizer.optimize(&ast)?;

        // Teste Optimierungen
        assert!(optimized_plan.has_index_usage(), "Should use indices");
        assert!(optimized_plan.estimated_cost() < 1000.0, "Cost should be reasonable");

        // Teste Neuromorphic Optimierungen
        if optimized_plan.can_use_synaptic_pathways() {
            assert!(optimized_plan.synaptic_efficiency() > 0.5);
            println!("✅ Using synaptic pathway optimization");
        }

        println!("✅ Query optimization works correctly");
        Ok(())
    }
}

/// Tests für Security und Authentication
mod security_tests {
    use super::*;
    use neuroquantum_core::security::{SecurityManager, EncryptionKey, BiometricAuth};

    #[test]
    fn test_quantum_encryption() -> Result<()> {
        let security_manager = SecurityManager::new();

        // Teste Quantum Key Distribution
        let (public_key, private_key) = security_manager.generate_quantum_keypair()?;

        let test_data = b"Sensitive medical data";
        let encrypted = security_manager.quantum_encrypt(test_data, &public_key)?;
        let decrypted = security_manager.quantum_decrypt(&encrypted, &private_key)?;

        assert_eq!(test_data, &decrypted[..]);

        // Teste Quantum-sichere Eigenschaften
        assert!(security_manager.is_quantum_resistant(&encrypted)?);

        println!("✅ Quantum encryption works correctly");
        Ok(())
    }

    #[test]
    fn test_biometric_authentication() -> Result<()> {
        let mut biometric_auth = BiometricAuth::new();

        // Simuliere EEG-basierte Authentifizierung
        let user_id = "patient_001";
        let eeg_pattern = vec![1.2, 3.4, 2.1, 4.5, 1.8]; // Simulierte EEG-Daten

        // Registriere biometrisches Template
        biometric_auth.register_eeg_template(user_id, &eeg_pattern)?;

        // Teste Authentifizierung
        let similar_pattern = vec![1.1, 3.5, 2.0, 4.4, 1.9]; // Leicht unterschiedlich
        let auth_result = biometric_auth.authenticate_eeg(user_id, &similar_pattern)?;

        assert!(auth_result.is_authenticated, "Authentication should succeed");
        assert!(auth_result.confidence > 0.8, "Confidence should be high");

        // Teste Reject bei unterschiedlichem Muster
        let different_pattern = vec![5.0, 6.0, 7.0, 8.0, 9.0];
        let reject_result = biometric_auth.authenticate_eeg(user_id, &different_pattern)?;

        assert!(!reject_result.is_authenticated, "Should reject different pattern");

        println!("✅ Biometric authentication works correctly");
        Ok(())
    }

    #[test]
    fn test_access_control() -> Result<()> {
        let security_manager = SecurityManager::new();

        // Teste Role-based Access Control
        security_manager.create_role("doctor", vec!["read_patients", "write_diagnoses"])?;
        security_manager.create_role("nurse", vec!["read_patients"])?;
        security_manager.create_role("admin", vec!["read_patients", "write_patients", "manage_users"])?;

        let user_id = "dr_smith";
        security_manager.assign_role(user_id, "doctor")?;

        // Teste Berechtigungen
        assert!(security_manager.has_permission(user_id, "read_patients")?);
        assert!(security_manager.has_permission(user_id, "write_diagnoses")?);
        assert!(!security_manager.has_permission(user_id, "manage_users")?);

        println!("✅ Access control works correctly");
        Ok(())
    }
}

/// Tests für Monitoring und Metriken
mod monitoring_tests {
    use super::*;
    use neuroquantum_core::monitoring::{MetricsCollector, PerformanceMonitor};

    #[test]
    fn test_metrics_collection() -> Result<()> {
        let mut collector = MetricsCollector::new();

        // Sammle verschiedene Metriken
        collector.record_query_time("SELECT", 45)?;
        collector.record_query_time("INSERT", 23)?;
        collector.record_query_time("UPDATE", 67)?;

        collector.record_throughput("inserts_per_second", 1500.0)?;
        collector.record_memory_usage(512 * 1024 * 1024)?; // 512 MB

        // Teste Metrik-Abruf
        let query_metrics = collector.get_query_metrics()?;
        assert_eq!(query_metrics.total_queries, 3);
        assert_eq!(query_metrics.average_time, 45.0); // (45+23+67)/3

        let throughput = collector.get_throughput_metric("inserts_per_second")?;
        assert_eq!(throughput, 1500.0);

        println!("✅ Metrics collection works correctly");
        Ok(())
    }

    #[test]
    fn test_performance_monitoring() -> Result<()> {
        let monitor = PerformanceMonitor::new();

        // Simuliere Datenbankoperationen
        monitor.start_operation("complex_query")?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        monitor.end_operation("complex_query")?;

        // Überprüfe Performance-Metriken
        let perf_report = monitor.generate_report()?;
        assert!(perf_report.operations.contains_key("complex_query"));

        let query_stats = &perf_report.operations["complex_query"];
        assert!(query_stats.average_duration_ms >= 100.0);
        assert_eq!(query_stats.execution_count, 1);

        println!("✅ Performance monitoring works correctly");
        Ok(())
    }

    #[test]
    fn test_resource_monitoring() -> Result<()> {
        let monitor = PerformanceMonitor::new();

        // Teste Ressourcen-Überwachung
        let initial_resources = monitor.get_resource_usage()?;
        assert!(initial_resources.cpu_usage >= 0.0);
        assert!(initial_resources.memory_usage > 0);

        // Simuliere Last
        let _dummy_data: Vec<u8> = vec![0; 10 * 1024 * 1024]; // 10 MB

        let loaded_resources = monitor.get_resource_usage()?;
        assert!(loaded_resources.memory_usage >= initial_resources.memory_usage);

        println!("✅ Resource monitoring works correctly");
        Ok(())
    }
}
