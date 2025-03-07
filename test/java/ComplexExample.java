package com.example.test;

import java.util.*;
import java.io.IOException;
import java.util.concurrent.CompletableFuture;
import java.util.stream.Collectors;

/**
 * A more complex Java test class with multiple features to test our Java parser.
 * This class demonstrates inheritance, interfaces, generics, and annotations.
 */
public class ComplexExample<T extends Comparable<T>> implements Iterable<T> {
    private final List<T> items;
    private final String name;
    private static final int MAX_SIZE = 100;
    
    @SuppressWarnings("unused")
    private Map<String, List<T>> groupedItems;
    
    /**
     * Creates a new complex example
     * @param name The example name
     * @param initialItems Initial items to add
     */
    public ComplexExample(String name, List<T> initialItems) {
        this.name = name;
        this.items = new ArrayList<>(initialItems);
        this.groupedItems = new HashMap<>();
    }
    
    /**
     * Add an item to the collection
     * @param item The item to add
     * @return true if added successfully
     * @throws IllegalStateException if collection is full
     */
    public boolean addItem(T item) throws IllegalStateException {
        if (items.size() >= MAX_SIZE) {
            throw new IllegalStateException("Collection is full");
        }
        return items.add(item);
    }
    
    /**
     * Groups items by a key function
     * @param keyExtractor The function to extract keys
     * @param <K> The key type
     * @return Map of items grouped by key
     */
    public <K> Map<K, List<T>> groupBy(KeyExtractor<T, K> keyExtractor) {
        return items.stream()
            .collect(Collectors.groupingBy(keyExtractor::extractKey));
    }
    
    /**
     * Performs an async operation on all items
     * @param processor The async processor
     * @return Future with processed results
     */
    public CompletableFuture<List<String>> processAsync(AsyncProcessor<T> processor) {
        List<CompletableFuture<String>> futures = items.stream()
            .map(processor::process)
            .collect(Collectors.toList());
            
        return CompletableFuture.allOf(
                futures.toArray(new CompletableFuture[0])
            )
            .thenApply(v -> 
                futures.stream()
                    .map(CompletableFuture::join)
                    .collect(Collectors.toList())
            );
    }
    
    /**
     * Key extractor interface for grouping
     */
    @FunctionalInterface
    public interface KeyExtractor<T, K> {
        K extractKey(T item);
    }
    
    /**
     * Async processor interface
     */
    @FunctionalInterface
    public interface AsyncProcessor<T> {
        CompletableFuture<String> process(T item);
    }
    
    /**
     * Iterator implementation
     */
    @Override
    public Iterator<T> iterator() {
        return items.iterator();
    }
    
    /**
     * Inner class example
     */
    public class ItemProcessor {
        public void processItems() throws IOException {
            for (T item : items) {
                System.out.println("Processing: " + item);
            }
        }
    }
    
    /**
     * Static nested class example
     */
    public static class Statistics {
        public static <U extends Number> double calculateAverage(List<U> numbers) {
            return numbers.stream()
                .mapToDouble(Number::doubleValue)
                .average()
                .orElse(0.0);
        }
    }
    
    /**
     * Main method to demonstrate usage
     */
    public static void main(String[] args) {
        List<Integer> numbers = Arrays.asList(1, 2, 3, 4, 5);
        ComplexExample<Integer> example = new ComplexExample<>("Test", numbers);
        
        example.addItem(6);
        
        Map<Boolean, List<Integer>> evenOddGroups = example.groupBy(n -> n % 2 == 0);
        
        System.out.println("Even numbers: " + evenOddGroups.get(true));
        System.out.println("Odd numbers: " + evenOddGroups.get(false));
        
        double average = Statistics.calculateAverage(numbers);
        System.out.println("Average: " + average);
        
        example.processAsync(n -> 
            CompletableFuture.supplyAsync(() -> "Processed " + n)
        ).thenAccept(results -> 
            results.forEach(System.out::println)
        );
    }
}