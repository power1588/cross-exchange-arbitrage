#!/bin/bash

# Cross-Exchange Arbitrage - 推送到远程仓库脚本
# 使用方法: ./push_to_remote.sh <repository_url>

echo "🚀 Cross-Exchange Arbitrage - Git 远程推送脚本"
echo "=============================================="

# 检查参数
if [ $# -eq 0 ]; then
    echo "❌ 错误: 请提供远程仓库URL"
    echo ""
    echo "使用方法:"
    echo "  ./push_to_remote.sh <repository_url>"
    echo ""
    echo "示例:"
    echo "  ./push_to_remote.sh https://github.com/username/cross-exchange-arbitrage.git"
    echo "  ./push_to_remote.sh https://gitlab.com/username/cross-exchange-arbitrage.git"
    exit 1
fi

REPO_URL=$1

echo "📋 项目信息:"
echo "  项目名称: Cross-Exchange Arbitrage Strategy"
echo "  本地路径: $(pwd)"
echo "  远程仓库: $REPO_URL"
echo ""

# 检查Git仓库状态
echo "🔍 检查Git状态..."
if [ ! -d ".git" ]; then
    echo "❌ 错误: 当前目录不是Git仓库"
    exit 1
fi

# 检查是否有未提交的更改
if [ -n "$(git status --porcelain)" ]; then
    echo "⚠️  警告: 发现未提交的更改"
    git status --short
    echo ""
    read -p "是否继续推送? (y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "❌ 推送已取消"
        exit 1
    fi
fi

# 显示提交历史
echo "📚 最近的提交历史:"
git log --oneline -5
echo ""

# 添加远程仓库
echo "🔗 添加远程仓库..."
if git remote get-url origin >/dev/null 2>&1; then
    echo "⚠️  远程仓库已存在，更新URL..."
    git remote set-url origin "$REPO_URL"
else
    git remote add origin "$REPO_URL"
fi

echo "✅ 远程仓库已配置: $REPO_URL"

# 推送到远程仓库
echo ""
echo "📤 推送代码到远程仓库..."
echo "执行: git push -u origin main"

if git push -u origin main; then
    echo ""
    echo "🎉 推送成功!"
    echo "📊 项目统计:"
    echo "  总提交数: $(git rev-list --count HEAD)"
    echo "  总文件数: $(find . -name '*.rs' -o -name '*.toml' -o -name '*.md' | grep -v target | wc -l | tr -d ' ')"
    echo "  代码行数: $(find . -name '*.rs' | grep -v target | xargs wc -l | tail -1 | awk '{print $1}')"
    echo ""
    echo "🔗 远程仓库地址: $REPO_URL"
    echo "✅ 您的项目现在可以在远程仓库中访问了!"
else
    echo ""
    echo "❌ 推送失败!"
    echo "可能的原因:"
    echo "  1. 远程仓库URL不正确"
    echo "  2. 没有推送权限"
    echo "  3. 网络连接问题"
    echo "  4. 远程仓库已有内容冲突"
    echo ""
    echo "解决方案:"
    echo "  git pull origin main --allow-unrelated-histories"
    echo "  git push -u origin main"
    exit 1
fi

echo ""
echo "🎯 下一步建议:"
echo "1. 在远程仓库设置中添加项目描述"
echo "2. 创建 Issues 跟踪后续开发"
echo "3. 设置 CI/CD 自动化测试"
echo "4. 邀请协作者 (如果需要)"
echo ""
echo "✅ Git 版本管理设置完成!"
