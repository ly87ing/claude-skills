; ============================================================================
; N+1 问题检测查询
; ============================================================================
;
; 检测在循环内进行数据库/远程调用的模式
; 匹配 for, while, foreach 三种循环类型
;
; ============================================================================

; for 循环中的方法调用
(for_statement
    body: (_
        (expression_statement
            (method_invocation
                name: (identifier) @method_name
            ) @call
        )
    )
)

; while 循环中的方法调用
(while_statement
    body: (_
        (expression_statement
            (method_invocation
                name: (identifier) @method_name
            ) @call
        )
    )
)

; enhanced for (foreach) 循环中的方法调用
(enhanced_for_statement
    body: (_
        (expression_statement
            (method_invocation
                name: (identifier) @method_name
            ) @call
        )
    )
)
