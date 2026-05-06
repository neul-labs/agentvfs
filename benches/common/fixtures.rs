//! Data generators for benchmark fixtures.

#![allow(dead_code)]

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Content size presets.
pub const SMALL_FILE: usize = 1024; // 1KB
pub const MEDIUM_FILE: usize = 64 * 1024; // 64KB
pub const LARGE_FILE: usize = 1024 * 1024; // 1MB

/// Deterministic data generator for benchmarks.
pub struct DataGenerator {
    rng: SmallRng,
    counter: u64,
}

impl DataGenerator {
    /// Create a new generator with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            counter: 0,
        }
    }

    /// Generate random binary content.
    pub fn content(&mut self, size: usize) -> Vec<u8> {
        (0..size).map(|_| self.rng.gen()).collect()
    }

    /// Generate searchable text content with embedded keywords.
    pub fn searchable_text(&mut self, size: usize, keywords: &[&str]) -> String {
        let words = [
            "lorem",
            "ipsum",
            "dolor",
            "sit",
            "amet",
            "consectetur",
            "adipiscing",
        ];
        let mut text = String::with_capacity(size);

        while text.len() < size {
            // 10% chance to insert a keyword
            if !keywords.is_empty() && self.rng.gen_bool(0.1) {
                let keyword = keywords[self.rng.gen_range(0..keywords.len())];
                text.push_str(keyword);
            } else {
                let word = words[self.rng.gen_range(0..words.len())];
                text.push_str(word);
            }
            text.push(' ');
        }

        text.truncate(size);
        text
    }

    /// Generate a unique file path.
    pub fn path(&mut self, prefix: &str) -> String {
        self.counter += 1;
        let rand_suffix: u32 = self.rng.gen();
        format!("{}/file_{}_{}.txt", prefix, self.counter, rand_suffix)
    }

    /// Generate a unique directory path.
    pub fn dir_path(&mut self, prefix: &str) -> String {
        self.counter += 1;
        let rand_suffix: u32 = self.rng.gen();
        format!("{}/dir_{}_{}", prefix, self.counter, rand_suffix)
    }

    /// Generate a directory tree structure.
    pub fn directory_tree(&mut self, depth: usize, breadth: usize) -> Vec<String> {
        let mut paths = Vec::new();
        self.generate_tree_recursive(&mut paths, "", depth, breadth);
        paths
    }

    fn generate_tree_recursive(
        &mut self,
        paths: &mut Vec<String>,
        prefix: &str,
        depth: usize,
        breadth: usize,
    ) {
        if depth == 0 {
            return;
        }

        for i in 0..breadth {
            let dir = if prefix.is_empty() {
                format!("/dir_{}", i)
            } else {
                format!("{}/dir_{}", prefix, i)
            };
            paths.push(dir.clone());
            self.generate_tree_recursive(paths, &dir, depth - 1, breadth);
        }
    }

    /// Generate random file size within a range.
    pub fn file_size(&mut self, min: usize, max: usize) -> usize {
        self.rng.gen_range(min..max)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_generator_deterministic() {
        use super::DataGenerator;
        let mut gen1 = DataGenerator::new(42);
        let mut gen2 = DataGenerator::new(42);

        let content1 = gen1.content(100);
        let content2 = gen2.content(100);

        assert_eq!(content1, content2);
    }

    #[test]
    fn test_searchable_text_contains_keywords() {
        use super::DataGenerator;
        let mut gen = DataGenerator::new(42);
        let text = gen.searchable_text(1000, &["important", "benchmark"]);

        // With 10% chance per word, we should have some keywords
        assert!(text.contains("important") || text.contains("benchmark") || text.len() == 1000);
    }

    #[test]
    fn test_unique_paths() {
        use super::DataGenerator;
        let mut gen = DataGenerator::new(42);
        let path1 = gen.path("/test");
        let path2 = gen.path("/test");

        assert_ne!(path1, path2);
    }
}
