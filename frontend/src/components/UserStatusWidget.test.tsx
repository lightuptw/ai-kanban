import type { ReactElement } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import UserStatusWidget from "./UserStatusWidget";
import { useAuth } from "../hooks/useAuth";

vi.mock("../hooks/useAuth", () => ({
  useAuth: vi.fn(() => ({
    user: {
      id: "u1",
      username: "test",
      nickname: "TestUser",
      first_name: "",
      last_name: "",
      email: "test@test.com",
      tenant_id: "t1",
      avatar_url: null,
      profile_completed: true,
    },
    updateUser: vi.fn(),
    refreshUser: vi.fn(),
  })),
}));

vi.mock("../constants", () => ({ API_BASE_URL: "" }));

const renderWithTheme = (ui: ReactElement) => {
  const theme = createTheme();
  return render(<ThemeProvider theme={theme}>{ui}</ThemeProvider>);
};

describe("UserStatusWidget", () => {
  const mockedUseAuth = vi.mocked(useAuth);

  beforeEach(() => {
    mockedUseAuth.mockReturnValue({
      user: {
        id: "u1",
        username: "test",
        nickname: "TestUser",
        first_name: "",
        last_name: "",
        email: "test@test.com",
        tenant_id: "t1",
        avatar_url: null,
        profile_completed: true,
      },
      updateUser: vi.fn(),
      refreshUser: vi.fn(),
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders user nickname", () => {
    renderWithTheme(<UserStatusWidget />);

    expect(screen.getByText("TestUser")).toBeVisible();
  });

  it("renders initials when no avatar", () => {
    renderWithTheme(<UserStatusWidget />);

    expect(screen.getByText("T")).toBeVisible();
  });

  it("calls onClick when clicked", () => {
    const onClick = vi.fn();
    renderWithTheme(<UserStatusWidget onClick={onClick} />);

    fireEvent.click(screen.getByRole("button", { name: "Open profile" }));

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("renders with avatar when avatar_url provided", () => {
    mockedUseAuth.mockReturnValue({
      user: {
        id: "u1",
        username: "test",
        nickname: "TestUser",
        first_name: "",
        last_name: "",
        email: "test@test.com",
        tenant_id: "t1",
        avatar_url: "/avatars/u1.png",
        profile_completed: true,
      },
      updateUser: vi.fn(),
      refreshUser: vi.fn(),
    });

    const { container } = renderWithTheme(<UserStatusWidget />);
    const avatarImage = container.querySelector("img");

    expect(avatarImage).not.toBeNull();
    expect(avatarImage?.getAttribute("src")).toContain("/avatars/u1.png");
  });
});
