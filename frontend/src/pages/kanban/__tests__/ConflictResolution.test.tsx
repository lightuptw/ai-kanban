import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConflictFileViewer } from "../ConflictFileViewer";
import { ConflictResolutionPanel } from "../ConflictResolutionPanel";
import { ManualResolveEditor } from "../ManualResolveEditor";
import type { ConflictFile, ConflictDetail } from "../../../types/kanban";

vi.mock("../../../services/api", () => ({
  api: {
    resolveConflicts: vi.fn().mockResolvedValue({ files: [], merge_in_progress: true }),
    completeMerge: vi.fn().mockResolvedValue({ success: true, message: "Merged", conflicts: [] }),
    abortMerge: vi.fn().mockResolvedValue(undefined),
  },
}));

const mockTextFile: ConflictFile = {
  path: "src/main.rs",
  ours_content: 'fn main() {\n    println!("ours");\n}',
  theirs_content: 'fn main() {\n    println!("theirs");\n}',
  base_content: 'fn main() {\n    println!("base");\n}',
  conflict_type: "both-modified",
  is_binary: false,
};

const mockBinaryFile: ConflictFile = {
  path: "image.png",
  ours_content: null,
  theirs_content: null,
  base_content: null,
  conflict_type: "both-modified",
  is_binary: true,
};

const mockDeletedByUsFile: ConflictFile = {
  path: "deleted.rs",
  ours_content: null,
  theirs_content: "some content",
  base_content: "original",
  conflict_type: "deleted-by-us",
  is_binary: false,
};

const mockConflictDetail: ConflictDetail = {
  files: [mockTextFile, mockBinaryFile],
  merge_in_progress: true,
};

describe("ConflictFileViewer", () => {
  it("renders ours and theirs content for text files", () => {
    render(<ConflictFileViewer file={mockTextFile} />);
    expect(screen.getByText("Ours (Current)")).toBeInTheDocument();
    expect(screen.getByText("Theirs (Incoming)")).toBeInTheDocument();
  });

  it("shows binary file message for binary conflicts", () => {
    render(<ConflictFileViewer file={mockBinaryFile} />);
    expect(screen.getByText("Binary file - choose Ours or Theirs")).toBeInTheDocument();
  });

  it("shows delete conflict message for deleted files", () => {
    render(<ConflictFileViewer file={mockDeletedByUsFile} />);
    expect(screen.getByText("Delete conflict detected")).toBeInTheDocument();
    expect(
      screen.getByText("Ours deleted this file while Theirs modified or kept it.")
    ).toBeInTheDocument();
  });
});

describe("ConflictResolutionPanel", () => {
  const mockOnComplete = vi.fn();
  const mockOnAbort = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders file list with conflict types", () => {
    render(
      <ConflictResolutionPanel
        cardId="test-card"
        conflictDetail={mockConflictDetail}
        onMergeComplete={mockOnComplete}
        onMergeAbort={mockOnAbort}
      />
    );
    expect(screen.getByText("src/main.rs")).toBeInTheDocument();
    expect(screen.getByText("image.png")).toBeInTheDocument();
  });

  it("shows progress indicator", () => {
    render(
      <ConflictResolutionPanel
        cardId="test-card"
        conflictDetail={mockConflictDetail}
        onMergeComplete={mockOnComplete}
        onMergeAbort={mockOnAbort}
      />
    );
    expect(screen.getByText("0 of 2 resolved")).toBeInTheDocument();
  });

  it("disables Complete Merge when not all resolved", () => {
    render(
      <ConflictResolutionPanel
        cardId="test-card"
        conflictDetail={mockConflictDetail}
        onMergeComplete={mockOnComplete}
        onMergeAbort={mockOnAbort}
      />
    );
    const completeBtn = screen.getByRole("button", { name: /complete merge/i });
    expect(completeBtn).toBeDisabled();
  });

  it("shows resolution action buttons for selected file", () => {
    render(
      <ConflictResolutionPanel
        cardId="test-card"
        conflictDetail={mockConflictDetail}
        onMergeComplete={mockOnComplete}
        onMergeAbort={mockOnAbort}
      />
    );
    expect(screen.getByRole("button", { name: /accept ours/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /accept theirs/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /edit manually/i })).toBeInTheDocument();
  });

  it("shows Abort Merge button", () => {
    render(
      <ConflictResolutionPanel
        cardId="test-card"
        conflictDetail={mockConflictDetail}
        onMergeComplete={mockOnComplete}
        onMergeAbort={mockOnAbort}
      />
    );
    const abortBtn = screen.getByRole("button", { name: /abort merge/i });
    expect(abortBtn).toBeInTheDocument();
    expect(abortBtn).not.toBeDisabled();
  });
});

describe("ManualResolveEditor", () => {
  it("renders with pre-populated content from theirs", () => {
    const mockResolve = vi.fn();
    const mockCancel = vi.fn();

    render(
      <ManualResolveEditor
        open={true}
        file={mockTextFile}
        onResolve={mockResolve}
        onCancel={mockCancel}
      />
    );

    expect(screen.getByText("Manually Resolve: main.rs")).toBeInTheDocument();
    const textbox = screen.getByRole("textbox");
    expect(textbox).toBeInTheDocument();
  });

  it("calls onCancel when cancel clicked", () => {
    const mockResolve = vi.fn();
    const mockCancel = vi.fn();

    render(
      <ManualResolveEditor
        open={true}
        file={mockTextFile}
        onResolve={mockResolve}
        onCancel={mockCancel}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(mockCancel).toHaveBeenCalled();
  });

  it("calls onResolve with content when Apply Resolution clicked", () => {
    const mockResolve = vi.fn();
    const mockCancel = vi.fn();

    render(
      <ManualResolveEditor
        open={true}
        file={mockTextFile}
        onResolve={mockResolve}
        onCancel={mockCancel}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /apply resolution/i }));
    expect(mockResolve).toHaveBeenCalledWith(mockTextFile.theirs_content);
  });
});
