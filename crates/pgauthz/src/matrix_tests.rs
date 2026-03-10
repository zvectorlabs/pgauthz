// YAML-based matrix testing framework for pgauthz
//
// This module provides a comprehensive testing framework inspired by OpenFGA's matrix tests,
// enabling systematic testing of complex authorization scenarios using declarative YAML definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// Represents a complete matrix test with model definition and test cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixTest {
    /// Name of the test matrix
    pub name: String,
    /// Authorization model DSL definition
    pub model: String,
    /// List of test cases to execute
    pub tests: Vec<TestCase>,
}

/// Individual test case with setup and assertions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Name of the test case
    pub name: String,
    /// Optional description of what this test covers
    pub description: Option<String>,
    /// Setup phase - tuples to create before running assertions
    pub setup: Option<TestSetup>,
    /// Assertions to validate after setup
    pub assertions: Vec<Assertion>,
}

/// Setup phase for creating test data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSetup {
    /// Tuples to create for the test
    pub tuples: Vec<TestTuple>,
}

/// Represents a tuple in the authorization model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTuple {
    /// Object in format "type:id"
    pub object: String,
    /// Relation name
    pub relation: String,
    /// Subject in format "type:id" or "type:id#relation"
    pub subject: String,
    /// Optional condition for conditional relationships
    pub condition: Option<String>,
}

/// Different types of assertions to validate authorization behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Assertion {
    /// Check if a subject has permission on an object
    Check {
        /// Object in format "type:id"
        object: String,
        /// Relation/permission to check
        relation: String,
        /// Subject in format "type:id"
        subject: String,
        /// Expected result (allowed/denied)
        allowed: bool,
        /// Optional context for conditional checks
        context: Option<HashMap<String, serde_json::Value>>,
    },
    /// Expand a relationship to see all reachable subjects
    Expand {
        /// Object in format "type:id"
        object: String,
        /// Relation to expand
        relation: String,
        /// Expected subjects that should be reachable
        subjects: Vec<String>,
    },
    /// List objects that a subject can access
    ListObjects {
        /// Subject in format "type:id"
        subject: String,
        /// Relation to check
        relation: String,
        /// Object type to filter
        object_type: String,
        /// Expected objects that should be accessible
        objects: Vec<String>,
    },
    /// List subjects that have access to an object
    ListSubjects {
        /// Object in format "type:id"
        object: String,
        /// Relation to check
        relation: String,
        /// Subject type to filter
        subject_type: String,
        /// Expected subjects that should have access
        subjects: Vec<String>,
    },
    /// Validate model parsing and structure
    Model {
        /// Should the model parse successfully
        valid: bool,
        /// Expected error message if invalid
        error: Option<String>,
    },
}

/// Error types for matrix testing
#[derive(Debug)]
pub enum MatrixTestError {
    YamlParse(serde_yaml::Error),
    Execution(String),
    Assertion(String),
}

impl Display for MatrixTestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::YamlParse(e) => write!(f, "YAML parsing error: {}", e),
            Self::Execution(e) => write!(f, "Test execution error: {}", e),
            Self::Assertion(e) => write!(f, "Assertion failed: {}", e),
        }
    }
}

impl std::error::Error for MatrixTestError {}

impl From<serde_yaml::Error> for MatrixTestError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::YamlParse(value)
    }
}

/// Result type for matrix test operations
pub type MatrixTestResult<T> = Result<T, MatrixTestError>;

/// Convert TestTuple to PgRelationship format for use with existing test helpers
impl From<&TestTuple> for crate::PgRelationship {
    fn from(tuple: &TestTuple) -> Self {
        // Parse object "type:id"
        let mut object_parts = tuple.object.split(':');
        let object_type = object_parts.next().unwrap_or("").to_string();
        let object_id = object_parts.next().unwrap_or("").to_string();

        // Parse subject "type:id" or "type:id#relation"
        let mut subject_parts = tuple.subject.split('#');
        let subject_base = subject_parts.next().unwrap_or("").to_string();
        let subject_relation = subject_parts.next().map(|s| s.to_string());
        let mut subject_base_parts = subject_base.split(':');
        let subject_type = subject_base_parts.next().unwrap_or("").to_string();
        let subject_base_id = subject_base_parts.next().unwrap_or("").to_string();

        // Include relation in subject_id if present
        let subject_id = match subject_relation {
            Some(rel) => format!("{}#{}", subject_base_id, rel),
            None => subject_base_id,
        };

        crate::PgRelationship {
            object_type,
            object_id,
            relation: tuple.relation.clone(),
            subject_type,
            subject_id,
            condition: tuple.condition.clone(),
        }
    }
}

/// Helper function to parse object type and ID from "type:id" format
pub fn parse_object_id(object: &str) -> (String, String) {
    let mut parts = object.split(':');
    let object_type = parts.next().unwrap_or("").to_string();
    let object_id = parts.next().unwrap_or("").to_string();
    (object_type, object_id)
}

/// Helper function to parse subject type and ID from "type:id" format
pub fn parse_subject_id(subject: &str) -> (String, String, Option<String>) {
    let mut parts = subject.split('#');
    let base = parts.next().unwrap_or("").to_string();
    let relation = parts.next().map(|s| s.to_string());

    let mut base_parts = base.split(':');
    let subject_type = base_parts.next().unwrap_or("").to_string();
    let subject_id = base_parts.next().unwrap_or("").to_string();

    (subject_type, subject_id, relation)
}

#[cfg(test)]
mod matrix_types_tests {
    use super::*;

    #[test]
    fn test_parse_object_id() {
        let (obj_type, obj_id) = parse_object_id("document:doc1");
        assert_eq!(obj_type, "document");
        assert_eq!(obj_id, "doc1");
    }

    #[test]
    fn test_parse_subject_id() {
        let (subj_type, subj_id, relation) = parse_subject_id("user:alice");
        assert_eq!(subj_type, "user");
        assert_eq!(subj_id, "alice");
        assert_eq!(relation, None);

        let (subj_type, subj_id, relation) = parse_subject_id("group:eng#member");
        assert_eq!(subj_type, "group");
        assert_eq!(subj_id, "eng");
        assert_eq!(relation, Some("member".to_string()));
    }

    #[test]
    fn test_test_tuple_conversion() {
        let test_tuple = TestTuple {
            object: "document:doc1".to_string(),
            relation: "viewer".to_string(),
            subject: "user:alice".to_string(),
            condition: None,
        };

        let pg_relationship: crate::PgRelationship = (&test_tuple).into();
        assert_eq!(pg_relationship.object_type, "document");
        assert_eq!(pg_relationship.object_id, "doc1");
        assert_eq!(pg_relationship.relation, "viewer");
        assert_eq!(pg_relationship.subject_type, "user");
        assert_eq!(pg_relationship.subject_id, "alice");
        assert_eq!(pg_relationship.condition, None);
    }
}
