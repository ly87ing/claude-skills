#!/bin/bash

# ============================================
# Java Performance Diagnostics - 卸载脚本
# ============================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════╗"
echo "║  Java Performance Diagnostics Uninstaller  ║"
echo "║  Java 性能诊断工具 - 卸载                   ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"

# 移除 MCP
echo -e "${YELLOW}[1/2] 移除 MCP Server...${NC}"
if command -v claude &> /dev/null; then
    claude mcp remove java-perf 2>/dev/null && echo -e "${GREEN}✓ MCP Server 已移除${NC}" || echo -e "${YELLOW}⚠ MCP Server 未注册或已移除${NC}"
else
    echo -e "${YELLOW}⚠ claude 命令未找到，请手动移除:${NC}"
    echo "   claude mcp remove java-perf"
fi

# 移除 Skill
echo ""
echo -e "${YELLOW}[2/2] 移除 Skill...${NC}"
SKILL_TARGET="$HOME/.claude/skills/java-perf"
if [ -d "$SKILL_TARGET" ]; then
    rm -rf "$SKILL_TARGET"
    echo -e "${GREEN}✓ Skill 已移除${NC}"
else
    echo -e "${YELLOW}⚠ Skill 未安装或已移除${NC}"
fi

# 完成
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════╗"
echo "║           ✓ 卸载完成！                     ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
