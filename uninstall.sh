#!/bin/bash

# ============================================
# Java Perf v6.0.0 - 卸载脚本
# ============================================
#
# Plugin 模式：推荐使用 /plugin uninstall java-perf
# 此脚本用于手动卸载（清理二进制文件）

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════╗"
echo "║  Java Perf v6.0.0 Uninstaller              ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"

# 移除二进制文件
echo -e "${YELLOW}[1/1] 移除二进制文件...${NC}"
INSTALL_DIR="$HOME/.local/bin"
if [ -f "$INSTALL_DIR/java-perf" ]; then
    rm "$INSTALL_DIR/java-perf"
    echo -e "${GREEN}✓ $INSTALL_DIR/java-perf 已移除${NC}"
else
    echo -e "${GREEN}✓ 二进制文件不存在${NC}"
fi

# 完成
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════╗"
echo "║           ✓ 卸载完成！                     ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo -e "${YELLOW}提示：如果通过 Plugin 安装，请使用 /plugin uninstall java-perf${NC}"
echo ""
