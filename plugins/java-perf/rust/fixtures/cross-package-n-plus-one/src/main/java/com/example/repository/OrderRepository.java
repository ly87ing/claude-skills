package com.example.repository;

import org.springframework.stereotype.Repository;
import java.util.List;

/**
 * Repository in com.example.repository package
 * This is the DAO layer that performs database operations
 */
@Repository
public class OrderRepository {

    public List<Order> findAll() {
        // Simulated database query
        return List.of();
    }

    public Order findById(Long id) {
        // Simulated database query
        return null;
    }

    public List<OrderItem> findItemsByOrderId(Long orderId) {
        // This method is called in a loop - N+1 problem
        return List.of();
    }
}
