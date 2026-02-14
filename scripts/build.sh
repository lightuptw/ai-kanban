#!/bin/bash
set -e

echo "Building Kanban V2..."
echo ""

echo "Step 1/2: Building frontend..."
cd frontend && npm run build
cd ..

echo ""
echo "Step 2/2: Building backend (release mode)..."
cd backend && cargo build --release
cd ..

echo ""
echo "✅ Build complete!"
echo ""
echo "To run the production server:"
echo "  ./backend/target/release/kanban-backend"
echo ""
echo "The server will serve both API and frontend at http://localhost:3000"
