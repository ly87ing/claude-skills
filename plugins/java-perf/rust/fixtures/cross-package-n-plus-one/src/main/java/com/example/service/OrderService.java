package com.example.service;

import org.springframework.stereotype.Service;
import com.example.repository.OrderRepository;
import java.util.List;

/**
 * Service in com.example.service package
 * Calls OrderRepository in com.example.repository package
 */
@Service
public class OrderService {

    private final OrderRepository orderRepository;

    public OrderService(OrderRepository orderRepository) {
        this.orderRepository = orderRepository;
    }

    public List<Order> findAllWithDetails() {
        List<Order> orders = orderRepository.findAll();
        // N+1 problem: loop query inside service
        for (Order order : orders) {
            order.setItems(orderRepository.findItemsByOrderId(order.getId()));
        }
        return orders;
    }
    
    public Order findById(Long id) {
        return orderRepository.findById(id);
    }
}
