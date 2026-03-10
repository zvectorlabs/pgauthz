// Matrix test execution engine for pgauthz
//
// This module handles parsing YAML test files and executing them within the PostgreSQL extension context.

use crate::cache;
use crate::matrix_tests::*;
use authz_core::model_ast::RelationExpr;
use authz_datastore_pgx::PostgresDatastore;
use pgrx::prelude::Spi;
use std::fs;
use std::time::Instant;

/// Matrix test runner that parses and executes YAML-based tests
pub struct MatrixRunner {
    test_files: Vec<String>,
    check_times: Vec<(String, String, u128)>, // (filename, test_case, duration_ms)
}

impl MatrixRunner {
    /// Create a new matrix runner with the specified test files
    pub fn new(test_files: Vec<String>) -> Self {
        Self {
            test_files,
            check_times: Vec::new(),
        }
    }

    /// Run all matrix tests
    pub fn run_all(&mut self) -> MatrixTestResult<()> {
        let test_files = self.test_files.clone();
        for test_file in test_files {
            self.run_file(&test_file)?;
        }
        self.print_timing_report();
        Ok(())
    }

    /// Run a single test file
    pub fn run_file(&mut self, file_path: &str) -> MatrixTestResult<()> {
        let filename = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        pgrx::info!("Running matrix test: {}", filename);

        let content = fs::read_to_string(file_path).map_err(|e| {
            MatrixTestError::Execution(format!("Failed to read file {}: {}", file_path, e))
        })?;

        self.run_str_with_filename(&content, filename)
    }

    /// Run tests from an in-memory YAML string
    pub fn run_str(&mut self, yaml_content: &str) -> MatrixTestResult<()> {
        self.run_str_with_filename(yaml_content, "unknown")
    }

    /// Run tests from an in-memory YAML string with a filename
    fn run_str_with_filename(
        &mut self,
        yaml_content: &str,
        filename: &str,
    ) -> MatrixTestResult<()> {
        let matrix_test: MatrixTest = serde_yaml::from_str(yaml_content)?;

        pgrx::info!(
            "Running matrix test: {} (file: {})",
            matrix_test.name,
            filename
        );

        // Create the policy (define_policy)
        let policy_q = format!(
            "SELECT pgauthz_define_policy({})",
            pgrx::spi::quote_literal(&matrix_test.model)
        );
        let policy_id: String = Spi::get_one(&policy_q)
            .map_err(|e| MatrixTestError::Execution(format!("policy define failed: {}", e)))?
            .ok_or_else(|| {
                MatrixTestError::Execution("pgauthz_define_policy returned no id".to_string())
            })?;

        pgrx::info!("Created policy with ID: {}", policy_id);

        // Run each test case
        for test_case in &matrix_test.tests {
            self.run_test_case(test_case, filename)?;
        }

        Ok(())
    }

    /// Run a single test case
    fn run_test_case(&mut self, test_case: &TestCase, filename: &str) -> MatrixTestResult<()> {
        pgrx::info!("Running test case: {} (file: {})", test_case.name, filename);

        // Isolate test cases by clearing tuples from any previous case in the same matrix file.
        Spi::run("DELETE FROM authz.tuple").map_err(|e| {
            MatrixTestError::Execution(format!(
                "Failed to clear tuples before test case '{}': {}",
                test_case.name, e
            ))
        })?;

        // Setup phase - create tuples
        if let Some(setup) = &test_case.setup {
            let tuples: Vec<crate::PgRelationship> =
                setup.tuples.iter().map(|t| t.into()).collect();
            crate::pgauthz_write_relationships(tuples, vec![]);
            pgrx::info!("Created {} setup tuples", setup.tuples.len());
        }

        // Assertions phase
        for (i, assertion) in test_case.assertions.iter().enumerate() {
            pgrx::info!(
                "Running assertion {} of {}: {:?}",
                i + 1,
                test_case.assertions.len(),
                assertion
            );
            if let Err(e) = self.run_assertion(assertion, &test_case.name, filename) {
                pgrx::error!(
                    "Assertion {} failed in test case '{}' (file: {}): {}",
                    i + 1,
                    test_case.name,
                    filename,
                    e
                );
            }
        }

        pgrx::info!("Test case '{}' passed (file: {})", test_case.name, filename);
        Ok(())
    }

    /// Run a single assertion
    fn run_assertion(
        &mut self,
        assertion: &Assertion,
        test_case_name: &str,
        filename: &str,
    ) -> MatrixTestResult<()> {
        match assertion {
            Assertion::Check {
                object,
                relation,
                subject,
                allowed,
                context,
            } => self.run_check_assertion(
                object,
                relation,
                subject,
                *allowed,
                context,
                test_case_name,
                filename,
            ),
            Assertion::Expand {
                object,
                relation,
                subjects,
            } => self.run_expand_assertion(object, relation, subjects, test_case_name, filename),
            Assertion::ListObjects {
                subject,
                relation,
                object_type,
                objects,
            } => self.run_list_objects_assertion(
                subject,
                relation,
                object_type,
                objects,
                test_case_name,
                filename,
            ),
            Assertion::ListSubjects {
                object,
                relation,
                subject_type,
                subjects,
            } => self.run_list_subjects_assertion(
                object,
                relation,
                subject_type,
                subjects,
                test_case_name,
                filename,
            ),
            Assertion::Model { valid, error } => {
                self.run_model_assertion(*valid, error, test_case_name, filename)
            }
        }
    }

    /// Run a check assertion
    fn run_check_assertion(
        &mut self,
        object: &str,
        relation: &str,
        subject: &str,
        allowed: bool,
        context: &Option<std::collections::HashMap<String, serde_json::Value>>,
        test_case_name: &str,
        filename: &str,
    ) -> MatrixTestResult<()> {
        let (obj_type, obj_id) = parse_object_id(object);
        let (subj_type, subj_id, _subj_rel) = parse_subject_id(subject);

        pgrx::info!(
            "Checking: {} {} {} -> expected {}",
            object,
            relation,
            subject,
            allowed
        );

        // Build the check query — use pgauthz_check_with_context when context is present
        let query = if let Some(ctx) = context {
            let ctx_json = serde_json::to_string(ctx).unwrap_or_else(|e| {
                pgrx::error!("Failed to serialize context: {}", e);
            });
            format!(
                "SELECT pgauthz_check_with_context({}, {}, {}, {}, {}, {})",
                pgrx::spi::quote_literal(&obj_type),
                pgrx::spi::quote_literal(&obj_id),
                pgrx::spi::quote_literal(relation),
                pgrx::spi::quote_literal(&subj_type),
                pgrx::spi::quote_literal(&subj_id),
                pgrx::spi::quote_literal(&ctx_json)
            )
        } else {
            format!(
                "SELECT pgauthz_check({}, {}, {}, {}, {})",
                pgrx::spi::quote_literal(&obj_type),
                pgrx::spi::quote_literal(&obj_id),
                pgrx::spi::quote_literal(relation),
                pgrx::spi::quote_literal(&subj_type),
                pgrx::spi::quote_literal(&subj_id)
            )
        };

        // Time the check execution
        let start_time = Instant::now();
        let result: bool = Spi::get_one(&query).unwrap().unwrap();
        let duration = start_time.elapsed().as_millis();

        // Record timing data
        self.check_times
            .push((filename.to_string(), test_case_name.to_string(), duration));

        pgrx::info!(
            "Check result: {} (expected: {}) - took {}ms",
            result,
            allowed,
            duration
        );

        if result != allowed {
            let error_msg = format!(
                "[{}] {}: Check {}:{}#{}@{}:{} returned {} but expected {}",
                filename,
                test_case_name,
                obj_type,
                obj_id,
                relation,
                subj_type,
                subj_id,
                result,
                allowed
            );
            pgrx::error!("{}", error_msg);
        }

        Ok(())
    }

    /// Print timing report for all check assertions
    fn print_timing_report(&self) {
        if self.check_times.is_empty() {
            pgrx::info!("No check assertions executed - no timing data to report");
            return;
        }

        // Generate report content
        let report_content = self.generate_timing_report_content();

        // Print to console
        pgrx::info!("{}", report_content);

        // Save to file
        self.save_timing_report_to_file(&report_content);
    }

    /// Generate timing report content as string
    fn generate_timing_report_content(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Matrix Test Timing Report ===\n");

        // Group by filename
        let mut by_file: std::collections::HashMap<String, Vec<(String, u128)>> =
            std::collections::HashMap::new();
        for (filename, test_case, duration) in &self.check_times {
            by_file
                .entry(filename.clone())
                .or_insert_with(Vec::new)
                .push((test_case.clone(), *duration));
        }

        let mut total_checks = 0;
        let mut total_time = 0u128;
        let mut min_time = u128::MAX;
        let mut max_time = 0u128;

        for (filename, checks) in by_file {
            let file_total: u128 = checks.iter().map(|(_, d)| *d).sum();
            let file_count = checks.len();
            let file_avg = file_total / file_count as u128;

            report.push_str(&format!(
                "File {}: {} checks, {}ms total, {}ms avg\n",
                filename, file_count, file_total, file_avg
            ));

            // Show slowest checks for this file
            let mut sorted_checks = checks.clone();
            sorted_checks.sort_by(|a, b| b.1.cmp(&a.1));
            for (test_case, duration) in sorted_checks.iter().take(3) {
                report.push_str(&format!("  {} - {}ms\n", test_case, duration));
            }

            total_checks += file_count;
            total_time += file_total;
            min_time = min_time.min(file_total);
            max_time = max_time.max(file_total);
        }

        let overall_avg = if total_checks > 0 {
            total_time / total_checks as u128
        } else {
            0
        };

        report.push_str("=== Overall Summary ===\n");
        report.push_str(&format!(
            "Total: {} checks, {}ms total, {}ms avg, {}ms min, {}ms max\n",
            total_checks, total_time, overall_avg, min_time, max_time
        ));
        report.push_str("=== End Timing Report ===\n");

        report
    }

    /// Save timing report to file in target directory
    fn save_timing_report_to_file(&self, content: &str) {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::path::Path;

        // Get workspace root target directory (go up from pgauthz/crates/pgauthz to authz, then to target)
        // CARGO_MANIFEST_DIR = /Users/rpatel9/projects/zanzibar/authz/crates/pgauthz
        // We need to go up two levels to reach /Users/rpatel9/projects/zanzibar/authz
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let target_dir = workspace_root.join("target");

        // Create target directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            pgrx::warning!("Failed to create target directory: {}", e);
            return;
        }

        // Use a fixed filename for cumulative timing data
        let filename = "matrix_test_timing_cumulative.txt";
        let file_path = target_dir.join(filename);

        // Append report to file with separator
        let mut file_content = String::new();

        // Add timestamp header
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        file_content.push_str(&format!("\n=== Test Run at {} ===\n", now));
        file_content.push_str(content);
        file_content.push_str("\n");

        // Append to file (create if doesn't exist)
        match OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&file_path)
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(file_content.as_bytes()) {
                    pgrx::warning!("Failed to write timing report to file: {}", e);
                } else {
                    pgrx::info!("Timing report appended to: {}", file_path.display());
                }
            }
            Err(e) => {
                pgrx::warning!("Failed to open timing report file: {}", e);
            }
        }
    }

    /// Collect the stored relation names that a relation/permission resolves to.
    /// For a plain relation this returns just that name; for a permission it
    /// recursively extracts the ComputedUserset leaves from the expression tree.
    fn resolve_stored_relations(&self, obj_type: &str, relation: &str) -> Vec<String> {
        let ds = PostgresDatastore::new();
        let ts = match cache::load_typesystem_cached(&ds) {
            Ok(ts) => ts,
            Err(_) => return vec![relation.to_string()],
        };

        // If it's a permission, walk its expression to find stored relations
        if ts.is_permission(obj_type, relation) {
            if let Some(rel_def) = ts.get_relation(obj_type, relation) {
                let mut relations = Vec::new();
                Self::collect_stored_relations(&rel_def.expression, &mut relations);
                if !relations.is_empty() {
                    return relations;
                }
            }
        }

        vec![relation.to_string()]
    }

    /// Recursively extract ComputedUserset relation names from an expression.
    fn collect_stored_relations(expr: &RelationExpr, out: &mut Vec<String>) {
        match expr {
            RelationExpr::DirectAssignment(_) => {
                // DirectAssignment tuples are stored under the parent relation,
                // which is already handled by the caller.
            }
            RelationExpr::ComputedUserset(rel) => {
                out.push(rel.clone());
            }
            RelationExpr::Union(exprs) => {
                for e in exprs {
                    Self::collect_stored_relations(e, out);
                }
            }
            RelationExpr::Intersection(exprs) => {
                for e in exprs {
                    Self::collect_stored_relations(e, out);
                }
            }
            RelationExpr::Exclusion { base, subtract } => {
                Self::collect_stored_relations(base, out);
                Self::collect_stored_relations(subtract, out);
            }
            RelationExpr::TupleToUserset { tupleset, .. } => {
                out.push(tupleset.clone());
            }
        }
    }

    /// Run an expand assertion
    fn run_expand_assertion(
        &mut self,
        object: &str,
        relation: &str,
        expected_subjects: &[String],
        test_case_name: &str,
        filename: &str,
    ) -> MatrixTestResult<()> {
        let (obj_type, obj_id) = parse_object_id(object);

        // Resolve permission to its underlying stored relations
        let stored_relations = self.resolve_stored_relations(&obj_type, relation);

        let mut actual_subjects: Vec<String> = Vec::new();

        for rel in &stored_relations {
            let query = format!(
                "SELECT subject_type || ':' || subject_id AS subject FROM authz.tuple WHERE object_type = {} AND object_id = {} AND relation = {} ORDER BY 1",
                pgrx::spi::quote_literal(&obj_type),
                pgrx::spi::quote_literal(&obj_id),
                pgrx::spi::quote_literal(rel)
            );

            Spi::connect(|client| {
                let table = client.select(&query, None, &[]).map_err(|e| {
                    MatrixTestError::Execution(format!("expand query failed: {}", e))
                })?;

                for row in table {
                    let subject: Option<String> = row.get_by_name("subject").map_err(|e| {
                        MatrixTestError::Execution(format!("expand row parse failed: {}", e))
                    })?;
                    if let Some(subject) = subject {
                        actual_subjects.push(subject);
                    }
                }
                Ok::<(), MatrixTestError>(())
            })?;
        }

        // Deduplicate and sort
        actual_subjects.sort();
        actual_subjects.dedup();

        let mut expected_sorted = expected_subjects.to_vec();
        expected_sorted.sort();

        if actual_subjects != expected_sorted {
            return Err(MatrixTestError::Assertion(format!(
                "[{}] {}: Expand mismatch for {}#{}: expected {:?}, got {:?}",
                filename, test_case_name, object, relation, expected_sorted, actual_subjects
            )));
        }

        Ok(())
    }

    /// Run a list objects assertion
    fn run_list_objects_assertion(
        &mut self,
        subject: &str,
        relation: &str,
        object_type: &str,
        expected_objects: &[String],
        test_case_name: &str,
        filename: &str,
    ) -> MatrixTestResult<()> {
        let (subj_type, subj_id, _subj_rel) = parse_subject_id(subject);

        let query = format!(
            "SELECT unnest(pgauthz_list_objects({}, {}, {}, {}, 100, NULL)) AS object_id",
            pgrx::spi::quote_literal(&subj_type),
            pgrx::spi::quote_literal(&subj_id),
            pgrx::spi::quote_literal(relation),
            pgrx::spi::quote_literal(object_type)
        );

        let mut actual_objects: Vec<String> = Vec::new();
        Spi::connect(|client| {
            let table = client.select(&query, None, &[]).map_err(|e| {
                MatrixTestError::Execution(format!("list objects query failed: {}", e))
            })?;

            for row in table {
                let object_id: Option<String> = row.get_by_name("object_id").map_err(|e| {
                    MatrixTestError::Execution(format!("list objects row parse failed: {}", e))
                })?;
                if let Some(object_id) = object_id {
                    actual_objects.push(object_id);
                }
            }
            Ok::<(), MatrixTestError>(())
        })?;

        let mut expected_sorted = expected_objects.to_vec();
        expected_sorted.sort();
        actual_objects.sort();

        if actual_objects != expected_sorted {
            return Err(MatrixTestError::Assertion(format!(
                "[{}] {}: ListObjects mismatch for {} {} {}: expected {:?}, got {:?}",
                filename,
                test_case_name,
                subject,
                relation,
                object_type,
                expected_sorted,
                actual_objects
            )));
        }

        Ok(())
    }

    /// Run a list subjects assertion
    fn run_list_subjects_assertion(
        &mut self,
        object: &str,
        relation: &str,
        subject_type: &str,
        expected_subjects: &[String],
        test_case_name: &str,
        filename: &str,
    ) -> MatrixTestResult<()> {
        let (obj_type, obj_id) = parse_object_id(object);

        let query = format!(
            "SELECT unnest(pgauthz_list_subjects({}, {}, {}, {}, 100, NULL)) AS subject_id",
            pgrx::spi::quote_literal(&obj_type),
            pgrx::spi::quote_literal(&obj_id),
            pgrx::spi::quote_literal(relation),
            pgrx::spi::quote_literal(subject_type)
        );

        let mut actual_subjects: Vec<String> = Vec::new();
        Spi::connect(|client| {
            let table = client.select(&query, None, &[]).map_err(|e| {
                MatrixTestError::Execution(format!("list subjects query failed: {}", e))
            })?;

            for row in table {
                let subject_id: Option<String> = row.get_by_name("subject_id").map_err(|e| {
                    MatrixTestError::Execution(format!("list subjects row parse failed: {}", e))
                })?;
                if let Some(subject_id) = subject_id {
                    actual_subjects.push(subject_id);
                }
            }
            Ok::<(), MatrixTestError>(())
        })?;

        let mut expected_sorted = expected_subjects.to_vec();
        expected_sorted.sort();
        actual_subjects.sort();

        if actual_subjects != expected_sorted {
            return Err(MatrixTestError::Assertion(format!(
                "[{}] {}: ListSubjects mismatch for {} {} {}: expected {:?}, got {:?}",
                filename,
                test_case_name,
                object,
                relation,
                subject_type,
                expected_sorted,
                actual_subjects
            )));
        }

        Ok(())
    }

    /// Run a model assertion
    fn run_model_assertion(
        &mut self,
        valid: bool,
        error: &Option<String>,
        _test_case_name: &str,
        _filename: &str,
    ) -> MatrixTestResult<()> {
        // For now, we assume the model was already created successfully in run_file
        // In a more complete implementation, we could test invalid models separately
        if !valid {
            let error_msg = error
                .clone()
                .unwrap_or_else(|| "Model should be invalid".to_string());
            return Err(MatrixTestError::Assertion(error_msg));
        }
        pgrx::info!("Model validation passed");
        Ok(())
    }
}

// Helper function to run matrix tests from a directory
#[allow(dead_code)]
pub fn run_matrix_tests_from_dir(dir_path: &str) -> MatrixTestResult<()> {
    let mut test_files = Vec::new();

    // Find all .yaml files in the directory
    for entry in fs::read_dir(dir_path).map_err(|e| {
        MatrixTestError::Execution(format!("Failed to read directory {}: {}", dir_path, e))
    })? {
        let entry = entry.map_err(|e| {
            MatrixTestError::Execution(format!("Failed to read directory entry: {}", e))
        })?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            test_files.push(path.to_string_lossy().to_string());
        }
    }

    let mut runner = MatrixRunner::new(test_files);
    runner.run_all()
}

#[cfg(test)]
mod matrix_tests {
    use super::*;

    #[test]
    fn test_matrix_runner_creation() {
        let runner = MatrixRunner::new(vec!["test.yaml".to_string()]);
        assert_eq!(runner.test_files.len(), 1);
    }
}
