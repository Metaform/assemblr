// Copyright (c) 2025 Metaform Systems, Inc
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
//
// Contributors:
//      Metaform Systems, Inc. - initial API and implementation

use assemblr::dag::{Graph, Vertex};

// ============================================================================
// Basic Construction & Vertex Operations
// ============================================================================

#[test]
fn test_vertex_creation() {
    let vertex = Vertex::new("test".to_string(), 42);
    assert_eq!(vertex.id, "test");
    assert_eq!(vertex.value, 42);
    assert!(vertex.edges.is_empty());

    // Test Display trait
    let display_str = format!("{}", vertex);
    assert!(display_str.contains("test"));
    assert!(display_str.contains("0"));
}

#[test]
fn test_empty_graph() {
    let graph: Graph<i32> = Graph::new();
    assert_eq!(graph.vertices.len(), 0);

    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order.len(), 0);
}

#[test]
fn test_single_vertex_no_edges() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);

    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order, vec!["A"]);
}

#[test]
fn test_duplicate_vertex_ignored() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("A".to_string(), 999); // Should be ignored

    assert_eq!(graph.vertices.len(), 1);
    assert_eq!(graph.get_value("A"), Some(&1)); // Original value preserved
}

// ============================================================================
// Edge Operations & Edge Cases
// ============================================================================

#[test]
fn test_add_edge_nonexistent_vertex() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);

    // Try to add edge to non-existent vertex
    graph.add_edge("A", "B");
    graph.add_edge("X", "A");

    let vertex = graph.get_vertex("A").unwrap();
    assert!(vertex.edges.is_empty()); // No edges should be added
}

#[test]
fn test_duplicate_edge_ignored() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);

    graph.add_edge("A", "B");
    graph.add_edge("A", "B"); // Duplicate
    graph.add_edge("A", "B"); // Duplicate

    let vertex = graph.get_vertex("A").unwrap();
    assert_eq!(vertex.edges.len(), 1);
}

#[test]
fn test_self_loop_detected() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_edge("A", "A"); // Self-loop

    let result = graph.topological_sort();
    assert!(result.has_cycle);
    assert!(result.sorted_order.is_empty());
}

// ============================================================================
// Basic Tests
// ============================================================================

#[test]
fn test_basic_graph() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);

    graph.add_edge("A", "B");
    graph.add_edge("B", "C");

    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order, vec!["A", "B", "C"]);
}

#[test]
fn test_cycle_detection() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);

    graph.add_edge("A", "B");
    graph.add_edge("B", "C");
    graph.add_edge("C", "A");

    let result = graph.topological_sort();
    assert!(result.has_cycle);
    assert!(!result.cycle_path.is_empty());
}

// ============================================================================
// Getter Methods
// ============================================================================

#[test]
fn test_get_vertex() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);

    assert!(graph.get_vertex("A").is_some());
    assert_eq!(graph.get_vertex("A").unwrap().id, "A");
    assert!(graph.get_vertex("NonExistent").is_none());
}

#[test]
fn test_get_value() {
    let mut graph: Graph<String> = Graph::new();
    graph.add_vertex("A".to_string(), "value_a".to_string());

    assert_eq!(graph.get_value("A"), Some(&"value_a".to_string()));
    assert_eq!(graph.get_value("NonExistent"), None);
}

// ============================================================================
// Dependency Tracking
// ============================================================================

#[test]
fn test_get_dependents() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);

    graph.add_edge("A", "B");
    graph.add_edge("A", "C");

    let vertex_a = graph.get_vertex("A").unwrap();
    let dependents = graph.get_dependents(vertex_a);

    assert_eq!(dependents.len(), 2);
    assert!(dependents.contains(&"B".to_string()));
    assert!(dependents.contains(&"C".to_string()));
}

#[test]
fn test_get_dependencies() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);

    graph.add_edge("A", "C");
    graph.add_edge("B", "C");

    let dependencies = graph.get_dependencies("C");

    assert_eq!(dependencies.len(), 2);
    assert!(dependencies.contains(&"A".to_string()));
    assert!(dependencies.contains(&"B".to_string()));
}

#[test]
fn test_dependencies_complex_graph() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);
    graph.add_vertex("D".to_string(), 4);

    graph.add_edge("A", "B");
    graph.add_edge("B", "C");
    graph.add_edge("A", "C");
    graph.add_edge("C", "D");

    // C depends on A and B
    let c_deps = graph.get_dependencies("C");
    assert_eq!(c_deps, vec!["A".to_string(), "B".to_string()]);

    // B has A as dependent
    let vertex_b = graph.get_vertex("B").unwrap();
    let b_dependents = graph.get_dependents(vertex_b);
    assert_eq!(b_dependents, vec!["C".to_string()]);
}

// ============================================================================
// Topological Sort Scenarios
// ============================================================================

#[test]
fn test_diamond_dependency() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);
    graph.add_vertex("D".to_string(), 4);

    // Diamond: A -> B -> D
    //          A -> C -> D
    graph.add_edge("A", "B");
    graph.add_edge("A", "C");
    graph.add_edge("B", "D");
    graph.add_edge("C", "D");

    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order.len(), 4);

    // A must come first, D must come last
    assert_eq!(result.sorted_order[0], "A");
    assert_eq!(result.sorted_order[3], "D");

    // B and C can be in any order but both after A and before D
    let b_pos = result.sorted_order.iter().position(|x| x == "B").unwrap();
    let c_pos = result.sorted_order.iter().position(|x| x == "C").unwrap();
    assert!(b_pos > 0 && b_pos < 3);
    assert!(c_pos > 0 && c_pos < 3);
}

#[test]
fn test_multiple_independent_components() {
    let mut graph: Graph<i32> = Graph::new();

    // Component 1: A -> B
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_edge("A", "B");

    // Component 2: C -> D
    graph.add_vertex("C".to_string(), 3);
    graph.add_vertex("D".to_string(), 4);
    graph.add_edge("C", "D");

    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order.len(), 4);

    // A must come before B
    let a_pos = result.sorted_order.iter().position(|x| x == "A").unwrap();
    let b_pos = result.sorted_order.iter().position(|x| x == "B").unwrap();
    assert!(a_pos < b_pos);

    // C must come before D
    let c_pos = result.sorted_order.iter().position(|x| x == "C").unwrap();
    let d_pos = result.sorted_order.iter().position(|x| x == "D").unwrap();
    assert!(c_pos < d_pos);
}

#[test]
fn test_multiple_roots() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);

    // No edges - all vertices are roots
    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order.len(), 3);
}

#[test]
fn test_complex_graph_topological_sort() {
    let mut graph: Graph<i32> = Graph::new();

    // Create a more complex graph
    for i in 0..6 {
        graph.add_vertex(format!("V{}", i), i);
    }

    // V0 -> V1 -> V3 -> V5
    // V0 -> V2 -> V3
    // V2 -> V4 -> V5
    graph.add_edge("V0", "V1");
    graph.add_edge("V0", "V2");
    graph.add_edge("V1", "V3");
    graph.add_edge("V2", "V3");
    graph.add_edge("V2", "V4");
    graph.add_edge("V3", "V5");
    graph.add_edge("V4", "V5");

    let result = graph.topological_sort();
    assert!(!result.has_cycle);
    assert_eq!(result.sorted_order.len(), 6);

    // V0 must be first, V5 must be last
    assert_eq!(result.sorted_order[0], "V0");
    assert_eq!(result.sorted_order[5], "V5");
}

// ============================================================================
// Cycle Detection
// ============================================================================

#[test]
fn test_cycle_path_accuracy() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_vertex("C".to_string(), 3);

    graph.add_edge("A", "B");
    graph.add_edge("B", "C");
    graph.add_edge("C", "A");

    let result = graph.topological_sort();
    assert!(result.has_cycle);

    // Cycle path should contain all three vertices
    assert_eq!(result.cycle_path.len(), 4); // Start vertex is repeated at the end
    assert_eq!(result.cycle_path[0], result.cycle_path[3]); // First and last should be same
}

#[test]
fn test_longer_cycle() {
    let mut graph: Graph<i32> = Graph::new();

    // Create 5-vertex cycle
    for i in 0..5 {
        graph.add_vertex(format!("V{}", i), i);
    }

    graph.add_edge("V0", "V1");
    graph.add_edge("V1", "V2");
    graph.add_edge("V2", "V3");
    graph.add_edge("V3", "V4");
    graph.add_edge("V4", "V0"); // Close the cycle

    let result = graph.topological_sort();
    assert!(result.has_cycle);
    assert!(!result.cycle_path.is_empty());
    assert!(result.sorted_order.is_empty());
}

#[test]
fn test_cycle_in_disconnected_graph() {
    let mut graph: Graph<i32> = Graph::new();

    // Component 1: A -> B (no cycle)
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_edge("A", "B");

    // Component 2: C -> D -> E -> C (has cycle)
    graph.add_vertex("C".to_string(), 3);
    graph.add_vertex("D".to_string(), 4);
    graph.add_vertex("E".to_string(), 5);
    graph.add_edge("C", "D");
    graph.add_edge("D", "E");
    graph.add_edge("E", "C");

    let result = graph.topological_sort();
    assert!(result.has_cycle);
}

#[test]
fn test_multiple_cycles() {
    let mut graph: Graph<i32> = Graph::new();

    // Cycle 1: A -> B -> A
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_edge("A", "B");
    graph.add_edge("B", "A");

    // Cycle 2: C -> D -> C
    graph.add_vertex("C".to_string(), 3);
    graph.add_vertex("D".to_string(), 4);
    graph.add_edge("C", "D");
    graph.add_edge("D", "C");

    let result = graph.topological_sort();
    assert!(result.has_cycle);
    // Should detect at least one cycle
    assert!(!result.cycle_path.is_empty());
}

// ============================================================================
// Special Cases
// ============================================================================

#[test]
fn test_clone_graph() {
    let mut graph: Graph<i32> = Graph::new();
    graph.add_vertex("A".to_string(), 1);
    graph.add_vertex("B".to_string(), 2);
    graph.add_edge("A", "B");

    let cloned_graph = graph.clone();

    assert_eq!(graph.vertices.len(), cloned_graph.vertices.len());
    assert_eq!(graph.get_value("A"), cloned_graph.get_value("A"));
    assert_eq!(graph.get_value("B"), cloned_graph.get_value("B"));
}

#[test]
fn test_default_trait() {
    let graph: Graph<i32> = Graph::default();
    assert_eq!(graph.vertices.len(), 0);
}
