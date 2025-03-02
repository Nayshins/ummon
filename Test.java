package com.example.test;

import java.util.List;
import java.util.ArrayList;

/**
 * This is a test Java class to verify that our Java parser works correctly.
 */
public class Test {
    private String name;
    private int count;
    
    /**
     * Constructor for Test class
     * @param name The name value
     * @param count The count value
     */
    public Test(String name, int count) {
        this.name = name;
        this.count = count;
    }
    
    /**
     * Returns a greeting message
     * @return The greeting message
     */
    public String getGreeting() {
        return "Hello, " + name + "! Count: " + count;
    }
    
    /**
     * Increment the counter by a specific amount
     * @param amount The amount to increment by
     */
    public void incrementCount(int amount) {
        this.count += amount;
    }
    
    /**
     * A static helper method that creates a list of test objects
     * @param size The number of objects to create
     * @return A list of test objects
     */
    public static List<Test> createTestList(int size) {
        List<Test> tests = new ArrayList<>();
        for (int i = 0; i < size; i++) {
            tests.add(new Test("Test" + i, i));
        }
        return tests;
    }
    
    /**
     * Main method to demonstrate usage
     * @param args Command line arguments
     */
    public static void main(String[] args) {
        Test test = new Test("World", 42);
        System.out.println(test.getGreeting());
        
        test.incrementCount(10);
        System.out.println(test.getGreeting());
        
        List<Test> testList = createTestList(5);
        for (Test t : testList) {
            System.out.println(t.getGreeting());
        }
    }
}