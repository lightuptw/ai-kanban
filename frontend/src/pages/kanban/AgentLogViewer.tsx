import React, { useState, useEffect, useRef, useCallback } from "react";
import styled from "@emotion/styled";
import { Box, Typography, Chip, CircularProgress } from "@mui/material";
import { api } from "../../services/api";
import type { AgentLog } from "../../types/kanban";

interface AgentLogViewerProps {
  cardId: string;
  sessionId: string | null;
}

const LogContainer = styled(Box)`
  background: #1e1e1e;
  border-radius: 8px;
  height: 400px;
  overflow-y: auto;
  padding: 12px 16px;
  font-family: "JetBrains Mono", "Fira Code", "Cascadia Code", monospace;
  font-size: 13px;
  line-height: 1.6;
  position: relative;

  &::-webkit-scrollbar {
    width: 8px;
  }
  &::-webkit-scrollbar-track {
    background: #2a2a2a;
    border-radius: 4px;
  }
  &::-webkit-scrollbar-thumb {
    background: #555;
    border-radius: 4px;
  }
`;

const LogEntry = styled(Box)`
  padding: 4px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  display: flex;
  gap: 8px;
  align-items: flex-start;

  &:last-of-type {
    border-bottom: none;
  }
`;

const Timestamp = styled(Typography)`
  color: #6a9955;
  font-family: inherit;
  font-size: inherit;
  white-space: nowrap;
  flex-shrink: 0;
`;

const LogContent = styled(Typography)`
  color: #e0e0e0;
  font-family: inherit;
  font-size: inherit;
  white-space: pre-wrap;
  word-break: break-word;
  flex: 1;
`;

const StreamingContent = styled(LogContent)`
  color: #ce9178;
`;

const StatusBar = styled(Box)`
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
`;

const AGENT_COLORS: Record<string, string> = {
  build: "#4fc3f7",
  oracle: "#ce93d8",
  explore: "#81c784",
  librarian: "#ffb74d",
  hephaestus: "#ef5350",
  metis: "#aed581",
  momus: "#ff8a65",
};

function getAgentColor(agent: string): string {
  const lower = agent.toLowerCase();
  return AGENT_COLORS[lower] || "#90a4ae";
}

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

interface DisplayEntry {
  id: string;
  timestamp: string;
  agent: string | null;
  content: string;
  eventType: string;
  isStreaming?: boolean;
}

function processLogs(logs: AgentLog[]): DisplayEntry[] {
  const entries: DisplayEntry[] = [];
  let deltaBuffer = "";
  let deltaAgent: string | null = null;
  let deltaTimestamp = "";
  let deltaId = "";

  for (const log of logs) {
    if (log.event_type === "message.part.delta") {
      if (deltaBuffer === "") {
        deltaTimestamp = log.created_at;
        deltaId = log.id;
        deltaAgent = log.agent;
      }
      deltaBuffer += log.content;
    } else {
      if (deltaBuffer) {
        entries.push({
          id: deltaId,
          timestamp: deltaTimestamp,
          agent: deltaAgent,
          content: deltaBuffer,
          eventType: "message.part.delta",
          isStreaming: false,
        });
        deltaBuffer = "";
      }
      entries.push({
        id: log.id,
        timestamp: log.created_at,
        agent: log.agent,
        content: log.content,
        eventType: log.event_type,
      });
    }
  }

  if (deltaBuffer) {
    entries.push({
      id: deltaId,
      timestamp: deltaTimestamp,
      agent: deltaAgent,
      content: deltaBuffer,
      eventType: "message.part.delta",
      isStreaming: true,
    });
  }

  return entries;
}

export const AgentLogViewer: React.FC<AgentLogViewerProps> = ({
  cardId,
  sessionId,
}) => {
  const [logs, setLogs] = useState<AgentLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [connected, setConnected] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout>>();
  const reconnectAttemptsRef = useRef(0);
  const shouldAutoScroll = useRef(true);

  const scrollToBottom = useCallback(() => {
    if (containerRef.current && shouldAutoScroll.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, []);

  const handleScroll = useCallback(() => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    shouldAutoScroll.current = scrollHeight - scrollTop - clientHeight < 50;
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function fetchLogs() {
      try {
        const data = await api.getCardLogs(cardId);
        if (!cancelled) {
          setLogs(data);
          setLoading(false);
          requestAnimationFrame(scrollToBottom);
        }
      } catch (err) {
        console.error("Failed to fetch agent logs:", err);
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    fetchLogs();
    return () => {
      cancelled = true;
    };
  }, [cardId, scrollToBottom]);

  useEffect(() => {
    function connectWs() {
      const wsProtocol =
        window.location.protocol === "https:" ? "wss:" : "ws:";
      const wsHost = import.meta.env.VITE_API_URL
        ? new URL(import.meta.env.VITE_API_URL as string).host
        : `${window.location.hostname}:3000`;
      const wsUrl = `${wsProtocol}//${wsHost}/ws/logs/${cardId}`;

      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnected(true);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          if (message.type === "agentLogCreated" && message.log) {
            const newLog: AgentLog = message.log;
            setLogs((prev) => [...prev, newLog]);
            requestAnimationFrame(scrollToBottom);
          }
        } catch (err) {
          console.error("Failed to parse WebSocket message:", err);
        }
      };

      ws.onclose = () => {
        setConnected(false);
        wsRef.current = null;

        const attempts = reconnectAttemptsRef.current;
        const delay = Math.min(1000 * Math.pow(2, attempts), 30000);
        reconnectAttemptsRef.current = attempts + 1;

        reconnectTimeoutRef.current = setTimeout(connectWs, delay);
      };

      ws.onerror = () => {
        ws.close();
      };
    }

    connectWs();

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [cardId, scrollToBottom]);

  const displayEntries = processLogs(logs);

  return (
    <Box>
      <StatusBar>
        <Typography variant="caption" color="text.secondary">
          {sessionId && `Session: ${sessionId.slice(0, 8)}...`}
        </Typography>
        <Chip
          size="small"
          label={connected ? "Connected" : "Disconnected"}
          sx={{
            bgcolor: connected ? "rgba(76, 175, 80, 0.15)" : "rgba(244, 67, 54, 0.15)",
            color: connected ? "#66bb6a" : "#ef5350",
            fontWeight: 600,
            fontSize: 11,
            height: 24,
          }}
        />
      </StatusBar>

      <LogContainer ref={containerRef} onScroll={handleScroll}>
        {loading && (
          <Box
            sx={{
              display: "flex",
              justifyContent: "center",
              alignItems: "center",
              height: "100%",
            }}
          >
            <CircularProgress size={28} sx={{ color: "#6a9955" }} />
          </Box>
        )}

        {!loading && displayEntries.length === 0 && (
          <Typography
            sx={{
              color: "#555",
              fontFamily: "inherit",
              fontSize: "inherit",
              textAlign: "center",
              mt: 8,
            }}
          >
            No agent logs yet. Logs will appear here when the AI agent starts
            working.
          </Typography>
        )}

        {displayEntries.map((entry) => (
          <LogEntry key={entry.id}>
            <Timestamp>{formatTime(entry.timestamp)}</Timestamp>
            {entry.agent && (
              <Chip
                label={entry.agent}
                size="small"
                sx={{
                  bgcolor: `${getAgentColor(entry.agent)}22`,
                  color: getAgentColor(entry.agent),
                  fontFamily: "inherit",
                  fontSize: 11,
                  height: 20,
                  flexShrink: 0,
                }}
              />
            )}
            {entry.isStreaming || entry.eventType === "message.part.delta" ? (
              <StreamingContent>{entry.content}</StreamingContent>
            ) : (
              <LogContent>{entry.content}</LogContent>
            )}
          </LogEntry>
        ))}
      </LogContainer>
    </Box>
  );
};
