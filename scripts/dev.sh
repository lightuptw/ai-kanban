#!/bin/bash

echo "Starting Kanban V2 development servers..."
echo ""

cd backend && cargo watch -x "run --bin kanban-backend" &
BACKEND_PID=$!

cd ../frontend && npm run dev &
FRONTEND_PID=$!

echo ""
echo "✅ Development servers starting..."
echo ""
echo "Backend:  http://localhost:3000"
echo "Frontend: http://localhost:5173"
echo "OpenCode: http://localhost:4096 (start manually if needed: opencode serve --port 4096)"
echo ""
echo "Press Ctrl+C to stop all servers"
echo ""

cleanup() {
    echo ""
    echo "Stopping servers..."
    kill $BACKEND_PID $FRONTEND_PID 2>/dev/null
    exit 0
}

trap cleanup INT TERM

wait
