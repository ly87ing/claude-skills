; ============================================================================
; 并发问题检测查询
; ============================================================================
;
; 检测常见的并发和线程安全问题：
; - synchronized 方法
; - ReentrantLock 泄漏
; - 锁内 sleep
; - ThreadLocal 泄漏
;
; ============================================================================

; synchronized 方法检测
(method_declaration
    (modifiers
        "synchronized"
    ) @mods
    name: (identifier) @method_name
) @method

; ReentrantLock 声明检测
(local_variable_declaration
    type: (type_identifier) @type
    declarator: (variable_declarator
        name: (identifier) @var_name
    )
    (#eq? @type "ReentrantLock")
) @lock_decl

; Thread.sleep 在锁内检测
(method_invocation
    object: (identifier) @class_name
    name: (identifier) @method_name
    (#eq? @class_name "Thread")
    (#eq? @method_name "sleep")
) @sleep_call

; ThreadLocal 声明检测
(field_declaration
    type: (generic_type
        (type_identifier) @type
        (#eq? @type "ThreadLocal")
    )
    declarator: (variable_declarator
        name: (identifier) @field_name
    )
) @threadlocal_field
