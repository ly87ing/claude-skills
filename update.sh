#!/bin/bash

# ============================================
# Java Perf v6.0.0 - 更新脚本
# ============================================
#
# 用法:
#   ./update.sh

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════╗"
echo "║  Java Perf v6.0.0 Updater                  ║"
echo "║  CLI + Skill 模式                          ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"

# 检查是否是 git 仓库
if [ ! -d "$SCRIPT_DIR/.git" ]; then
    echo -e "${RED}❌ 当前目录不是 git 仓库，无法自动更新${NC}"
    echo "   请手动下载最新版本: https://github.com/ly87ing/java-perf-skill"
    exit 1
fi

# 拉取最新代码
echo -e "${YELLOW}[1/3] 拉取最新代码...${NC}"
cd "$SCRIPT_DIR"
git fetch origin
CURRENT_BRANCH=$(git branch --show-current)
BEHIND=$(git rev-list HEAD..origin/$CURRENT_BRANCH --count 2>/dev/null || echo "0")

if [ "$BEHIND" = "0" ]; then
    echo -e "${GREEN}✓ 已是最新版本${NC}"
else
    echo -e "${YELLOW}  发现 $BEHIND 个新提交，正在更新...${NC}"
    git pull origin $CURRENT_BRANCH
    echo -e "${GREEN}✓ 代码更新完成${NC}"
fi

# 更新二进制文件
echo ""
echo -e "${YELLOW}[2/3] 更新二进制文件...${NC}"

# 检测平台
PLATFORM=$(uname -s)
ARCH=$(uname -m)
case "$PLATFORM-$ARCH" in
    Darwin-arm64) BINARY="java-perf-darwin-arm64" ;;
    Darwin-x86_64) BINARY="java-perf-darwin-x64" ;;
    Linux-x86_64) BINARY="java-perf-linux-x64" ;;
    *) BINARY="" ;;
esac

UPDATE_SUCCESS=false
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

# 1. 优先使用本地编译版本
if [ -f "$SCRIPT_DIR/rust/target/release/java-perf" ]; then
    cp "$SCRIPT_DIR/rust/target/release/java-perf" "$INSTALL_DIR/java-perf"
    chmod +x "$INSTALL_DIR/java-perf"
    echo -e "${GREEN}✓ 使用本地编译版本${NC}"
    UPDATE_SUCCESS=true
fi

# 2. 尝试从 GitHub Release 下载
if [ "$UPDATE_SUCCESS" = "false" ] && [ -n "$BINARY" ] && command -v curl &> /dev/null; then
    echo "  尝试下载最新 Release..."
    REPO="ly87ing/java-perf-skill"
    RELEASE_URL="https://github.com/$REPO/releases/latest/download/$BINARY"

    if curl -fsSL "$RELEASE_URL" -o "$INSTALL_DIR/java-perf.tmp" 2>/dev/null; then
        chmod +x "$INSTALL_DIR/java-perf.tmp"
        mv "$INSTALL_DIR/java-perf.tmp" "$INSTALL_DIR/java-perf"
        echo -e "${GREEN}✓ 已下载最新二进制文件${NC}"
        UPDATE_SUCCESS=true
    else
        echo -e "${YELLOW}⚠ 下载失败，尝试本地编译...${NC}"
        rm -f "$INSTALL_DIR/java-perf.tmp"
    fi
fi

# 3. 如果下载失败，尝试本地编译
if [ "$UPDATE_SUCCESS" = "false" ]; then
    if command -v cargo &> /dev/null; then
        echo "  正在通过源码编译..."
        cd "$SCRIPT_DIR/rust"
        if cargo build --release; then
            cp target/release/java-perf "$INSTALL_DIR/java-perf"
            chmod +x "$INSTALL_DIR/java-perf"
            echo -e "${GREEN}✓ 编译并安装完成${NC}"
            UPDATE_SUCCESS=true
        else
            echo -e "${RED}❌ 编译失败${NC}"
            exit 1
        fi
        cd "$SCRIPT_DIR"
    else
        echo -e "${RED}❌ 更新失败：无法下载二进制文件，且未检测到 Rust 环境${NC}"
        exit 1
    fi
fi

# 更新 Skill
echo ""
echo -e "${YELLOW}[3/3] 更新 Skill...${NC}"
SKILL_SOURCE="$SCRIPT_DIR/skills/java-perf"
SKILL_TARGET="$HOME/.claude/skills/java-perf"

mkdir -p "$HOME/.claude/skills"
if [ -d "$SKILL_TARGET" ]; then
    rm -rf "$SKILL_TARGET"
fi
cp -r "$SKILL_SOURCE" "$SKILL_TARGET"
echo -e "${GREEN}✓ Skill 更新完成${NC}"

# 显示版本
echo ""
echo -e "${YELLOW}当前版本:${NC}"
"$INSTALL_DIR/java-perf" status 2>/dev/null || echo "  (无法获取版本)"

# 完成
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════╗"
echo "║           ✓ 更新完成！                     ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo "使用方式："
echo -e "  ${YELLOW}java-perf scan --path ./src${NC}"
echo -e "  ${YELLOW}java-perf status${NC}"
