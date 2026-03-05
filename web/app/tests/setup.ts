import { afterEach } from "vitest";

afterEach(() => {
  window.history.replaceState({}, "", "/");
});
