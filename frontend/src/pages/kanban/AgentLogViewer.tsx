import React, { useState, useEffect, useRef, useCallback, useMemo } from "react";
import styled from "@emotion/styled";
import { keyframes } from "@emotion/react";
import {
  Box,
  Typography,
  Chip,
  CircularProgress,
  Collapse,
  IconButton,
} from "@mui/material";
import { api } from "../../services/api";
import type { AgentLog } from "../../types/kanban";
import { API_BASE_URL } from "../../constants";

interface AgentLogViewerProps {
  cardId: string;
  sessionId: string | null;
  aiStatus?: string;
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

const statusPulse = keyframes`
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
`;

const AI_STATUS_CONFIG: Record<string, { color: string; bg: string; pulse: boolean }> = {
  planning: { color: "#42a5f5", bg: "rgba(66, 165, 245, 0.15)", pulse: true },
  working: { color: "#ffa726", bg: "rgba(255, 167, 38, 0.15)", pulse: true },
  dispatched: { color: "#ffee58", bg: "rgba(255, 238, 88, 0.15)", pulse: false },
  completed: { color: "#66bb6a", bg: "rgba(102, 187, 106, 0.15)", pulse: false },
  failed: { color: "#ef5350", bg: "rgba(239, 83, 80, 0.15)", pulse: false },
  idle: { color: "#90a4ae", bg: "rgba(144, 164, 174, 0.15)", pulse: false },
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
  isSubagent: boolean;
  agentType: string | null;
  isStreaming?: boolean;
}

interface AgentMetadata {
  _subagent?: boolean;
  _agent_type?: string;
}

type RenderItem =
  | { type: "entry"; key: string; entry: DisplayEntry }
  | {
      type: "group";
      key: string;
      agentKey: string;
      entries: DisplayEntry[];
      firstTimestamp: string;
      lastTimestamp: string;
    };

export function parseAgentMetadata(metadata: string): AgentMetadata {
  if (!metadata) {
    return {};
  }

  try {
    const parsed: unknown = JSON.parse(metadata);
    if (!parsed || typeof parsed !== "object") {
      return {};
    }

    const value = parsed as Record<string, unknown>;
    return {
      _subagent: value._subagent === true,
      _agent_type:
        typeof value._agent_type === "string" ? value._agent_type : undefined,
    };
  } catch {
    return {};
  }
}

export function getEntryAgentKey(entry: DisplayEntry): string | null {
  return entry.agent ?? entry.agentType;
}

export function processLogs(logs: AgentLog[]): DisplayEntry[] {
  const entries: DisplayEntry[] = [];
  let deltaBuffer = "";
  let deltaAgent: string | null = null;
  let deltaTimestamp = "";
  let deltaId = "";
  let deltaIsSubagent = false;
  let deltaAgentType: string | null = null;

  for (const log of logs) {
    const metadata = parseAgentMetadata(log.metadata);
    const isSubagent = metadata._subagent === true;
    const agentType = metadata._agent_type ?? null;

    if (log.event_type === "message.part.delta") {
      if (deltaBuffer === "") {
        deltaTimestamp = log.created_at;
        deltaId = log.id;
        deltaAgent = log.agent;
        deltaIsSubagent = isSubagent;
        deltaAgentType = agentType;
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
          isSubagent: deltaIsSubagent,
          agentType: deltaAgentType,
          isStreaming: false,
        });
        deltaBuffer = "";
        deltaAgent = null;
        deltaTimestamp = "";
        deltaId = "";
        deltaIsSubagent = false;
        deltaAgentType = null;
      }
      entries.push({
        id: log.id,
        timestamp: log.created_at,
        agent: log.agent,
        content: log.content,
        eventType: log.event_type,
        isSubagent,
        agentType,
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
      isSubagent: deltaIsSubagent,
      agentType: deltaAgentType,
      isStreaming: true,
    });
  }

  return entries;
}

export const AgentLogViewer: React.FC<AgentLogViewerProps> = ({
  cardId,
  sessionId,
  aiStatus,
}) => {
  const [logs, setLogs] = useState<AgentLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [connected, setConnected] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout>>();
  const reconnectAttemptsRef = useRef(0);
  const shouldAutoScroll = useRef(true);
  const [visibleAgents, setVisibleAgents] = useState<Record<string, boolean>>({});
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({});

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
      const apiUrl = new URL(API_BASE_URL);
      const wsProtocol = apiUrl.protocol === "https:" ? "wss:" : "ws:";
      const wsUrl = `${wsProtocol}//${apiUrl.host}/ws/logs/${cardId}`;

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

  const displayEntries = useMemo(() => processLogs(logs), [logs]);

  const uniqueAgents = useMemo(() => {
    const keys = new Set<string>();
    for (const entry of displayEntries) {
      const agentKey = getEntryAgentKey(entry);
      if (agentKey) {
        keys.add(agentKey);
      }
    }
    return Array.from(keys).sort((a, b) => a.localeCompare(b));
  }, [displayEntries]);

  useEffect(() => {
    setVisibleAgents((prev) => {
      const next: Record<string, boolean> = {};
      let changed = false;

      for (const agent of uniqueAgents) {
        next[agent] = prev[agent] ?? true;
        if (!(agent in prev)) {
          changed = true;
        }
      }

      if (!changed && Object.keys(prev).length === Object.keys(next).length) {
        return prev;
      }

      return next;
    });
  }, [uniqueAgents]);

  const filteredEntries = useMemo(() => {
    return displayEntries.filter((entry) => {
      const agentKey = getEntryAgentKey(entry);
      if (!agentKey) {
        return true;
      }
      return visibleAgents[agentKey] !== false;
    });
  }, [displayEntries, visibleAgents]);

  const renderItems = useMemo(() => {
    const items: RenderItem[] = [];
    let i = 0;

    while (i < filteredEntries.length) {
      const current = filteredEntries[i];
      const currentAgentKey = getEntryAgentKey(current);

      if (current.isSubagent && currentAgentKey) {
        const groupedEntries: DisplayEntry[] = [current];
        let j = i + 1;

        while (j < filteredEntries.length) {
          const candidate = filteredEntries[j];
          if (!candidate.isSubagent) {
            break;
          }

          const candidateAgentKey = getEntryAgentKey(candidate);
          if (candidateAgentKey !== currentAgentKey) {
            break;
          }

          groupedEntries.push(candidate);
          j += 1;
        }

        if (groupedEntries.length > 1) {
          items.push({
            type: "group",
            key: `group-${currentAgentKey}-${groupedEntries[0].id}`,
            agentKey: currentAgentKey,
            entries: groupedEntries,
            firstTimestamp: groupedEntries[0].timestamp,
            lastTimestamp: groupedEntries[groupedEntries.length - 1].timestamp,
          });
        } else {
          items.push({
            type: "entry",
            key: current.id,
            entry: current,
          });
        }

        i = j;
      } else {
        items.push({
          type: "entry",
          key: current.id,
          entry: current,
        });
        i += 1;
      }
    }

    return items;
  }, [filteredEntries]);

  useEffect(() => {
    setCollapsedGroups((prev) => {
      const next = { ...prev };
      let changed = false;

      for (const item of renderItems) {
        if (item.type === "group" && next[item.key] === undefined) {
          next[item.key] = true;
          changed = true;
        }
      }

      return changed ? next : prev;
    });
  }, [renderItems]);

  const toggleAgentVisibility = useCallback((agent: string) => {
    setVisibleAgents((prev) => ({
      ...prev,
      [agent]: !(prev[agent] ?? true),
    }));
  }, []);

  const toggleGroup = useCallback((groupKey: string) => {
    setCollapsedGroups((prev) => ({
      ...prev,
      [groupKey]: !(prev[groupKey] ?? true),
    }));
  }, []);

  const renderEntry = useCallback((entry: DisplayEntry, key?: string) => {
    const agentKey = getEntryAgentKey(entry);
    const borderColor = getAgentColor(agentKey ?? "");

    return (
      <LogEntry
        key={key ?? entry.id}
        sx={
          entry.isSubagent
            ? {
                ml: 2,
                pl: 2,
                borderLeft: `3px solid ${borderColor}`,
                borderBottomColor: "rgba(255, 255, 255, 0.08)",
              }
            : undefined
        }
      >
        <Timestamp>{formatTime(entry.timestamp)}</Timestamp>
        {entry.isSubagent && (
          <Typography
            component="span"
            sx={{ color: borderColor, lineHeight: 1.5, fontSize: 12, flexShrink: 0 }}
          >
            {"->"}
          </Typography>
        )}
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
    );
  }, []);

  return (
    <Box>
      <StatusBar>
        <Typography variant="caption" color="text.secondary">
          {sessionId && `Session: ${sessionId.slice(0, 8)}...`}
        </Typography>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          {(() => {
            const statusKey = aiStatus || "idle";
            const config = AI_STATUS_CONFIG[statusKey] || AI_STATUS_CONFIG.idle;
            return (
              <Chip
                size="small"
                label={statusKey}
                sx={{
                  bgcolor: config.bg,
                  color: config.color,
                  fontWeight: 600,
                  fontSize: 11,
                  height: 22,
                  animation: config.pulse ? `${statusPulse} 2s ease-in-out infinite` : 'none',
                }}
              />
            );
          })()}
          <Box
            title={connected ? "WS Connected" : "WS Disconnected"}
            sx={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              bgcolor: connected ? "#66bb6a" : "#ef5350",
              flexShrink: 0,
            }}
          />
        </Box>
      </StatusBar>

      {uniqueAgents.length > 0 && (
        <Box
          sx={{
            display: "flex",
            alignItems: "center",
            flexWrap: "wrap",
            gap: 0.75,
            mb: 1,
          }}
        >
          <Typography sx={{ color: "#9aa0a6", fontSize: 11, mr: 0.5 }}>
            Agents
          </Typography>
          {uniqueAgents.map((agent) => {
            const visible = visibleAgents[agent] !== false;
            const color = getAgentColor(agent);

            return (
              <Chip
                key={agent}
                label={agent}
                size="small"
                clickable
                onClick={() => toggleAgentVisibility(agent)}
                variant={visible ? "filled" : "outlined"}
                sx={{
                  height: 22,
                  fontFamily: "inherit",
                  fontSize: 11,
                  borderColor: `${color}66`,
                  bgcolor: visible ? `${color}26` : "transparent",
                  color: visible ? color : "#8d96a0",
                  opacity: visible ? 1 : 0.7,
                }}
              />
            );
          })}
        </Box>
      )}

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

        {renderItems.map((item) => {
          if (item.type === "entry") {
            return renderEntry(item.entry, item.key);
          }

          const isCollapsed = collapsedGroups[item.key] ?? true;
          const groupColor = getAgentColor(item.agentKey);

          return (
            <Box
              key={item.key}
              sx={{
                ml: 2,
                mb: 0.5,
                borderLeft: `3px solid ${groupColor}`,
                borderRadius: 0.5,
                background: "rgba(255, 255, 255, 0.01)",
              }}
            >
              <Box
                sx={{
                  display: "flex",
                  alignItems: "center",
                  px: 1,
                  py: 0.5,
                  borderBottom: isCollapsed ? "none" : "1px solid rgba(255, 255, 255, 0.08)",
                }}
              >
                <IconButton
                  size="small"
                  onClick={() => toggleGroup(item.key)}
                  sx={{ p: 0.25, mr: 0.5, color: groupColor }}
                >
                  <Typography sx={{ fontFamily: "inherit", fontSize: 12, lineHeight: 1 }}>
                    {isCollapsed ? "+" : "-"}
                  </Typography>
                </IconButton>
                <Typography sx={{ color: groupColor, fontSize: 11, fontWeight: 600, mr: 1 }}>
                  {"-> Delegated: "}{item.agentKey}
                </Typography>
                <Typography sx={{ color: "#8d96a0", fontSize: 11 }}>
                  {item.entries.length} logs ({formatTime(item.firstTimestamp)}-{formatTime(item.lastTimestamp)})
                </Typography>
              </Box>

              <Collapse in={!isCollapsed} timeout="auto" unmountOnExit>
                <Box sx={{ pl: 2 }}>
                  {item.entries.map((entry) => renderEntry(entry, `${item.key}-${entry.id}`))}
                </Box>
              </Collapse>
            </Box>
          );
        })}
      </LogContainer>
    </Box>
  );
};
