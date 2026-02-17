import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AgentLogViewer, getEntryAgentKey, parseAgentMetadata, processLogs } from "./AgentLogViewer";
import { api } from "../../services/api";
import type { AgentLog } from "../../types/kanban";

vi.mock("../../services/api", () => ({
  api: {
    getCardLogs: vi.fn(),
  },
}));

let mockIdCounter = 0;

function createMockLog(overrides: Partial<AgentLog> = {}): AgentLog {
  mockIdCounter += 1;
  return {
    id: `log-${mockIdCounter}`,
    card_id: "test-card-1",
    session_id: "ses_parent",
    event_type: "message.updated",
    agent: null,
    content: "test log content",
    metadata: "{}",
    created_at: "2026-02-16T10:00:00.000Z",
    ...overrides,
  };
}

class MockWebSocket {
  static instances: MockWebSocket[] = [];

  url: string;
  readyState = 0;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send = vi.fn();

  close = vi.fn(() => {
    this.readyState = 3;
    if (this.onclose) {
      this.onclose({} as CloseEvent);
    }
  });

  emitOpen() {
    this.readyState = 1;
    if (this.onopen) {
      this.onopen(new Event("open"));
    }
  }

  emitMessage(payload: unknown) {
    if (this.onmessage) {
      this.onmessage({ data: JSON.stringify(payload) } as MessageEvent);
    }
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  mockIdCounter = 0;
  MockWebSocket.instances = [];
  vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
});

describe("AgentLogViewer helpers", () => {
  it("parseAgentMetadata parses _subagent and _agent_type from metadata json", () => {
    expect(parseAgentMetadata('{"_subagent":true,"_agent_type":"explore"}')).toEqual({
      _subagent: true,
      _agent_type: "explore",
    });

    expect(parseAgentMetadata("not-json")).toEqual({});
    expect(parseAgentMetadata('{"_subagent":"yes","_agent_type":123}')).toEqual({
      _subagent: false,
      _agent_type: undefined,
    });
  });

  it("processLogs buffers deltas and sets sub-agent fields", () => {
    const logs: AgentLog[] = [
      createMockLog({
        id: "d1",
        event_type: "message.part.delta",
        content: "Hel",
        agent: "explore",
        metadata: '{"_subagent":true,"_agent_type":"explore"}',
        created_at: "2026-02-16T10:00:00.000Z",
      }),
      createMockLog({
        id: "d2",
        event_type: "message.part.delta",
        content: "lo",
        agent: "explore",
        metadata: '{"_subagent":true,"_agent_type":"explore"}',
        created_at: "2026-02-16T10:00:01.000Z",
      }),
      createMockLog({
        id: "m1",
        event_type: "message.updated",
        content: "Complete",
        agent: "build",
        metadata: "{}",
      }),
      createMockLog({
        id: "d3",
        event_type: "message.part.delta",
        content: "Tail",
        agent: "oracle",
        metadata: '{"_subagent":true,"_agent_type":"oracle"}',
        created_at: "2026-02-16T10:00:02.000Z",
      }),
      createMockLog({
        id: "d4",
        event_type: "message.part.delta",
        content: " end",
        agent: "oracle",
        metadata: '{"_subagent":true,"_agent_type":"oracle"}',
        created_at: "2026-02-16T10:00:03.000Z",
      }),
    ];

    const entries = processLogs(logs);

    expect(entries).toHaveLength(3);

    expect(entries[0]).toMatchObject({
      id: "d1",
      eventType: "message.part.delta",
      content: "Hello",
      agent: "explore",
      isSubagent: true,
      agentType: "explore",
      isStreaming: false,
    });

    expect(entries[1]).toMatchObject({
      id: "m1",
      eventType: "message.updated",
      content: "Complete",
      agent: "build",
      isSubagent: false,
      agentType: null,
    });

    expect(entries[2]).toMatchObject({
      id: "d3",
      eventType: "message.part.delta",
      content: "Tail end",
      agent: "oracle",
      isSubagent: true,
      agentType: "oracle",
      isStreaming: true,
    });
  });

  it("getEntryAgentKey returns entry.agent then falls back to entry.agentType", () => {
    expect(
      getEntryAgentKey({
        id: "a",
        timestamp: "2026-02-16T10:00:00.000Z",
        agent: "build",
        content: "x",
        eventType: "message.updated",
        isSubagent: false,
        agentType: "explore",
      })
    ).toBe("build");

    expect(
      getEntryAgentKey({
        id: "b",
        timestamp: "2026-02-16T10:00:00.000Z",
        agent: null,
        content: "y",
        eventType: "message.updated",
        isSubagent: true,
        agentType: "explore",
      })
    ).toBe("explore");
  });
});

describe("AgentLogViewer rendering", () => {
  it("renders sub-agent entry with indentation/border and correct agent chip color", async () => {
    vi.mocked(api.getCardLogs).mockResolvedValue([
      createMockLog({
        id: "s1",
        agent: "explore",
        metadata: '{"_subagent":true,"_agent_type":"explore"}',
        content: "↳ explore | Spawned: Find auth patterns",
      }),
    ]);

    render(<AgentLogViewer cardId="test-card-1" sessionId="ses_parent" aiStatus="working" />);

    await waitFor(() => {
      expect(screen.getByText("↳ explore | Spawned: Find auth patterns")).toBeInTheDocument();
    });

    const entry = screen
      .getByText("↳ explore | Spawned: Find auth patterns")
      .closest("div[class*='MuiBox-root']") as HTMLElement;
    const entryStyles = window.getComputedStyle(entry);

    expect(entryStyles.marginLeft).toBe("16px");
    expect(entryStyles.borderLeftStyle).toBe("solid");
    expect(entryStyles.borderLeftWidth).toBe("3px");
    expect(entryStyles.borderLeftColor).toMatch(/#81c784|rgb\(129,\s*199,\s*132\)/i);

    const exploreFilterChip = screen.getByRole("button", { name: "explore" });
    const chipStyles = window.getComputedStyle(exploreFilterChip);
    expect(chipStyles.color).toMatch(/#81c784|rgb\(129,\s*199,\s*132\)/i);
  });

  it("toggles agent filter chips to show and hide entries", async () => {
    vi.mocked(api.getCardLogs).mockResolvedValue([
      createMockLog({ id: "b1", agent: "build", content: "build output" }),
      createMockLog({
        id: "e1",
        agent: "explore",
        content: "explore output",
        metadata: '{"_subagent":true,"_agent_type":"explore"}',
      }),
    ]);

    render(<AgentLogViewer cardId="test-card-1" sessionId="ses_parent" aiStatus="working" />);

    await waitFor(() => {
      expect(screen.getByText("build output")).toBeInTheDocument();
      expect(screen.getByText("explore output")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "explore" }));

    await waitFor(() => {
      expect(screen.getByText("build output")).toBeInTheDocument();
      expect(screen.queryByText("explore output")).not.toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "explore" }));

    await waitFor(() => {
      expect(screen.getByText("explore output")).toBeInTheDocument();
    });
  });

  it("groups consecutive sub-agent entries into collapsible delegated sections", async () => {
    vi.mocked(api.getCardLogs).mockResolvedValue([
      createMockLog({
        id: "g1",
        agent: "explore",
        metadata: '{"_subagent":true,"_agent_type":"explore"}',
        content: "delegated step 1",
        created_at: "2026-02-16T10:00:00.000Z",
      }),
      createMockLog({
        id: "g2",
        agent: "explore",
        metadata: '{"_subagent":true,"_agent_type":"explore"}',
        content: "delegated step 2",
        created_at: "2026-02-16T10:00:10.000Z",
      }),
      createMockLog({
        id: "n1",
        agent: "build",
        metadata: "{}",
        content: "normal agent log",
        created_at: "2026-02-16T10:00:20.000Z",
      }),
    ]);

    render(<AgentLogViewer cardId="test-card-1" sessionId="ses_parent" aiStatus="working" />);

    await waitFor(() => {
      expect(screen.getByText(/-> Delegated: explore/)).toBeInTheDocument();
      expect(screen.getByText(/2 logs \(/)).toBeInTheDocument();
    });

    expect(screen.queryByText("delegated step 1")).not.toBeInTheDocument();
    expect(screen.queryByText("delegated step 2")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "+" }));

    await waitFor(() => {
      expect(screen.getByText("delegated step 1")).toBeInTheDocument();
      expect(screen.getByText("delegated step 2")).toBeInTheDocument();
      expect(screen.getByText("normal agent log")).toBeInTheDocument();
    });
  });
});
