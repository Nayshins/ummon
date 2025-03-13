use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::io::Write;
use tempfile::tempdir;
use ummon::parser::language_support::get_parser_for_file;

// Sample code snippets for benchmarking each language
const RUST_SAMPLE: &str = r#"
/// A simple function that adds two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// A structure to represent a point in 2D space
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    
    /// Calculate distance from origin
    pub fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    /// Calculate distance between two points
    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Calculate the sum of a vector of numbers
pub fn sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}
"#;

const PYTHON_SAMPLE: &str = r#"
"""
Module for demonstrating Python parsing capabilities
"""

def add(a, b):
    """Add two numbers and return the result"""
    return a + b

class Point:
    """A class to represent a point in 2D space"""
    
    def __init__(self, x, y):
        """Initialize with x and y coordinates"""
        self.x = x
        self.y = y
        
    def distance_from_origin(self):
        """Calculate distance from origin"""
        return (self.x**2 + self.y**2)**0.5
        
    def distance(self, other):
        """Calculate distance between two points"""
        dx = self.x - other.x
        dy = self.y - other.y
        return (dx**2 + dy**2)**0.5

def sum_numbers(numbers):
    """Calculate the sum of a list of numbers"""
    return sum(numbers)
    
# Create some points
p1 = Point(1.0, 2.0)
p2 = Point(4.0, 6.0)
"#;

const JAVASCRIPT_SAMPLE: &str = r#"
/**
 * A module for demonstrating JavaScript parsing capabilities
 */

/**
 * Add two numbers and return the result
 * @param {number} a - First number
 * @param {number} b - Second number
 * @returns {number} The sum of a and b
 */
function add(a, b) {
    return a + b;
}

/**
 * A class to represent a point in 2D space
 */
class Point {
    /**
     * Initialize with x and y coordinates
     * @param {number} x - X coordinate
     * @param {number} y - Y coordinate
     */
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    
    /**
     * Calculate distance from origin
     * @returns {number} Distance from origin
     */
    distanceFromOrigin() {
        return Math.sqrt(this.x**2 + this.y**2);
    }
    
    /**
     * Calculate distance between two points
     * @param {Point} other - The other point
     * @returns {number} Distance between points
     */
    distance(other) {
        const dx = this.x - other.x;
        const dy = this.y - other.y;
        return Math.sqrt(dx**2 + dy**2);
    }
}

/**
 * Calculate the sum of an array of numbers
 * @param {number[]} numbers - Array of numbers
 * @returns {number} Sum of the numbers
 */
function sumNumbers(numbers) {
    return numbers.reduce((sum, num) => sum + num, 0);
}

// Create some points
const p1 = new Point(1.0, 2.0);
const p2 = new Point(4.0, 6.0);
"#;

const JAVA_SAMPLE: &str = r#"
/**
 * A class for demonstrating Java parsing capabilities
 */
public class Point {
    private double x;
    private double y;
    
    /**
     * Initialize with x and y coordinates
     * @param x X coordinate
     * @param y Y coordinate
     */
    public Point(double x, double y) {
        this.x = x;
        this.y = y;
    }
    
    /**
     * Get the X coordinate
     * @return X coordinate
     */
    public double getX() {
        return x;
    }
    
    /**
     * Get the Y coordinate
     * @return Y coordinate
     */
    public double getY() {
        return y;
    }
    
    /**
     * Calculate distance from origin
     * @return Distance from origin
     */
    public double distanceFromOrigin() {
        return Math.sqrt(x*x + y*y);
    }
    
    /**
     * Calculate distance between two points
     * @param other The other point
     * @return Distance between points
     */
    public double distance(Point other) {
        double dx = this.x - other.x;
        double dy = this.y - other.y;
        return Math.sqrt(dx*dx + dy*dy);
    }
    
    /**
     * Static method to add two numbers
     * @param a First number
     * @param b Second number
     * @return Sum of a and b
     */
    public static int add(int a, int b) {
        return a + b;
    }
    
    /**
     * Calculate the sum of an array of numbers
     * @param numbers Array of numbers
     * @return Sum of the numbers
     */
    public static int sum(int[] numbers) {
        int result = 0;
        for (int num : numbers) {
            result += num;
        }
        return result;
    }
}
"#;

/// Helper function to create a temporary file for a specific language
fn create_temp_file(language: &str) -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    let (file_name, content) = match language {
        "rust" => ("sample.rs", RUST_SAMPLE),
        "python" => ("sample.py", PYTHON_SAMPLE),
        "javascript" => ("sample.js", JAVASCRIPT_SAMPLE),
        "java" => ("Sample.java", JAVA_SAMPLE),
        _ => panic!("Unsupported language: {}", language),
    };

    let file_path = dir.path().join(file_name);
    let mut file = fs::File::create(&file_path).expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to temp file");

    dir
}

/// Benchmark parsing performance for different languages
pub fn bench_parsing(c: &mut Criterion) {
    // Create a benchmark group for parser benchmarks
    let mut group = c.benchmark_group("parser_benchmarks");

    // Languages to benchmark
    let languages = vec!["rust", "python", "javascript", "java"];

    for lang in languages {
        // Create a temporary file for the language
        let temp_dir = create_temp_file(lang);
        let file_path = match lang {
            "rust" => temp_dir.path().join("sample.rs"),
            "python" => temp_dir.path().join("sample.py"),
            "javascript" => temp_dir.path().join("sample.js"),
            "java" => temp_dir.path().join("Sample.java"),
            _ => panic!("Unsupported language: {}", lang),
        };

        // Read the file content
        let content = fs::read_to_string(&file_path).expect("Failed to read temp file");
        let file_path_str = file_path.to_string_lossy().to_string();

        // Create a parser for the language
        let mut parser = get_parser_for_file(&file_path).expect("Failed to get parser");

        // Benchmark function parsing
        group.bench_with_input(
            BenchmarkId::new("parse_functions", lang),
            &lang,
            |b, _lang| {
                b.iter(|| parser.parse_functions(&content, &file_path_str).unwrap());
            },
        );

        // Benchmark type parsing
        group.bench_with_input(BenchmarkId::new("parse_types", lang), &lang, |b, _lang| {
            b.iter(|| parser.parse_types(&content, &file_path_str).unwrap());
        });

        // Benchmark call parsing
        group.bench_with_input(BenchmarkId::new("parse_calls", lang), &lang, |b, _lang| {
            b.iter(|| parser.parse_calls(&content).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parsing);
criterion_main!(benches);
