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

#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

const UNVISITED: u8 = 0;
const VISITING: u8 = 1;
const VISITED: u8 = 2;

/// A node in the graph
#[derive(Clone)]
pub struct Vertex<T: Clone> {
    pub id: String,
    pub value: T,
    pub edges: Vec<String>, // Store IDs instead of references for easier management
}

impl<T: Clone + fmt::Debug> fmt::Display for Vertex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vertex{{ID: {}, Edges: {}}}", self.id, self.edges.len())
    }
}

/// A directed acyclic graph
#[derive(Clone)]
pub struct Graph<T: Clone> {
    pub vertices: HashMap<String, Vertex<T>>,
}

/// Contains both the sorted order and any detected cycles
#[derive(Clone, Debug)]
pub struct SortResult {
    pub sorted_order: Vec<String>,
    pub has_cycle: bool,
    pub cycle_path: Vec<String>,
}

impl<T: Clone> Vertex<T> {
    /// Creates a new vertex with the given ID and value
    pub fn new(id: String, value: T) -> Self {
        Vertex {
            id,
            value,
            edges: Vec::new(),
        }
    }
}

impl<T: Clone> Graph<T> {
    /// Creates a new empty graph
    pub fn new() -> Self {
        Graph {
            vertices: HashMap::new(),
        }
    }

    /// Adds a vertex with the given ID and value to the graph
    pub fn add_vertex(&mut self, id: String, value: T) {
        if !self.vertices.contains_key(&id) {
            self.vertices.insert(id.clone(), Vertex::new(id, value));
        }
    }

    /// Adds a directed edge from vertex with ID `from_id` to vertex with ID `to_id`
    pub fn add_edge(&mut self, from_id: &str, to_id: &str) {
        // Check if both vertices exist
        if !self.vertices.contains_key(from_id) || !self.vertices.contains_key(to_id) {
            return;
        }

        // Get the from vertex and check if edge already exists
        if let Some(vertex) = self.vertices.get_mut(from_id) {
            if !vertex.edges.contains(&to_id.to_string()) {
                vertex.edges.push(to_id.to_string());
            }
        }
    }

    /// Returns the vertex with the given ID if it exists
    pub fn get_vertex(&self, id: &str) -> Option<&Vertex<T>> {
        self.vertices.get(id)
    }

    /// Performs topological sorting of the graph
    /// Returns a SortResult containing the sorted order and cycle information
    pub fn topological_sort(&self) -> SortResult {
        let mut result = SortResult {
            sorted_order: Vec::new(),
            has_cycle: false,
            cycle_path: Vec::new(),
        };

        // First check for cycles
        let (has_cycle, cycle_path) = self.detect_cycle_with_path();
        result.has_cycle = has_cycle;
        result.cycle_path = cycle_path;

        if has_cycle {
            return result;
        }

        // Calculate in-degree for all vertices
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize in-degree for all vertices
        for id in self.vertices.keys() {
            in_degree.insert(id.clone(), 0);
        }

        // Calculate in-degrees
        for vertex in self.vertices.values() {
            for edge_id in &vertex.edges {
                *in_degree.entry(edge_id.clone()).or_insert(0) += 1;
            }
        }

        // Create a queue and enqueue vertices with in-degree 0
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let total_vertices = self.vertices.len();
        let mut visited = 0;

        while !queue.is_empty() {
            // Dequeue a vertex
            let u_id = queue.remove(0);
            result.sorted_order.push(u_id.clone());
            visited += 1;

            // Get the edges for this vertex
            if let Some(vertex) = self.vertices.get(&u_id) {
                let edges = vertex.edges.clone();

                // Reduce in-degree of adjacent vertices
                for edge_id in edges {
                    if let Some(degree) = in_degree.get_mut(&edge_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(edge_id.clone());
                        }
                    }
                }
            }
        }

        // If visited count doesn't match vertices count, there's a cycle
        if visited != total_vertices {
            result.has_cycle = true;
            result.sorted_order.clear();
        }

        result
    }

    /// Returns the value associated with the given vertex ID
    pub fn get_value(&self, id: &str) -> Option<&T> {
        self.vertices.get(id).map(|v| &v.value)
    }

    /// Returns the dependents of a given vertex
    pub fn get_dependents(&self, vertex: &Vertex<T>) -> Vec<String> {
        let mut dependents = vertex.edges.clone();
        dependents.sort();
        dependents
    }

    /// Returns the dependencies of a given vertex
    pub fn get_dependencies(&self, vertex_id: &str) -> Vec<String> {
        let mut dependencies = Vec::new();

        // Find vertices that point to this one
        for vertex in self.vertices.values() {
            if vertex.edges.contains(&vertex_id.to_string()) {
                dependencies.push(vertex.id.clone());
            }
        }

        dependencies.sort();
        dependencies
    }

    /// Performs cycle detection using DFS and returns the cycle path if found
    fn detect_cycle_with_path(&self) -> (bool, Vec<String>) {
        let mut visit_state: HashMap<String, u8> = HashMap::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        // Initialize all vertices as unvisited
        for id in self.vertices.keys() {
            visit_state.insert(id.clone(), UNVISITED);
        }

        // Try DFS from each unvisited vertex
        for id in self.vertices.keys() {
            if *visit_state.get(id).unwrap_or(&UNVISITED) == UNVISITED {
                if let (true, cycle_path) = self.detect_cycle(id, &mut visit_state, &mut parent) {
                    return (true, cycle_path);
                }
            }
        }

        (false, Vec::new())
    }

    /// Performs DFS traversal to detect cycles and returns the cycle path if found
    fn detect_cycle(
        &self,
        vertex_id: &str,
        visit_state: &mut HashMap<String, u8>,
        parent: &mut HashMap<String, String>,
    ) -> (bool, Vec<String>) {
        visit_state.insert(vertex_id.to_string(), VISITING);

        if let Some(vertex) = self.vertices.get(vertex_id) {
            for neighbor_id in &vertex.edges {
                let state = *visit_state.get(neighbor_id).unwrap_or(&UNVISITED);

                if state == VISITING {
                    // Back-edge found - cycle detected
                    let mut cycle = vec![neighbor_id.clone()];
                    let mut current = vertex_id.to_string();

                    // Trace back from the current vertex to the start of the cycle
                    while current != *neighbor_id {
                        cycle.push(current.clone());
                        current = parent.get(&current).cloned().unwrap_or_default();
                    }

                    // Add the starting vertex again to complete the cycle
                    cycle.push(neighbor_id.clone());

                    // Reverse to get the correct order
                    cycle.reverse();

                    return (true, cycle);
                } else if state == UNVISITED {
                    parent.insert(neighbor_id.clone(), vertex_id.to_string());
                    if let (true, cycle_path) = self.detect_cycle(neighbor_id, visit_state, parent) {
                        return (true, cycle_path);
                    }
                }
            }
        }

        visit_state.insert(vertex_id.to_string(), VISITED);
        (false, Vec::new())
    }
}

impl<T: Clone> Default for Graph<T> {
    fn default() -> Self {
        Self::new()
    }
}