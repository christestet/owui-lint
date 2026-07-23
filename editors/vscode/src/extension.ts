import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let restartInFlight: Promise<void> | undefined;

const RELEASES_API =
  "https://api.github.com/repos/christestet/owui-lint/releases/latest";
const RELEASES_PAGE =
  "https://github.com/christestet/owui-lint/releases/latest";

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("owui-lint.restartServer", () => restart()),
    vscode.commands.registerCommand("owui-lint.showOutput", () => {
      client?.outputChannel.show();
    }),
    vscode.commands.registerCommand("owui-lint.checkForUpdates", () =>
      checkForUpdates(context),
    ),
    vscode.commands.registerCommand("owui-lint.updateCli", () => updateCli()),
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("owui-lint.enable")) {
        if (isEnabled()) {
          await start();
        } else {
          await stopClient();
        }
        return;
      }

      if (
        event.affectsConfiguration("owui-lint.path") ||
        event.affectsConfiguration("owui-lint.extraArgs")
      ) {
        await restart();
      }
    }),
  );

  await start();
}

export async function deactivate(): Promise<void> {
  await stopClient();
}

/** Stops and disposes the running language client, if any. */
async function stopClient(): Promise<void> {
  const current = client;
  client = undefined;
  await current?.dispose();
}

function createClient(): LanguageClient {
  const config = vscode.workspace.getConfiguration("owui-lint");
  const command = resolveExecutablePath(config.get<string>("path", "owui-lint"));
  const extraArgs = config.get<string[]>("extraArgs", []);

  const serverOptions: ServerOptions = {
    run: { command, args: ["server", ...extraArgs] },
    debug: { command, args: ["server", ...extraArgs] },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "python" },
      { scheme: "untitled", language: "python" },
    ],
  };

  return new LanguageClient("owui-lint", "owui-lint", serverOptions, clientOptions);
}

function isEnabled(): boolean {
  return vscode.workspace
    .getConfiguration("owui-lint")
    .get<boolean>("enable", true);
}

/**
 * Compares the running extension version against the latest GitHub release.
 * The extension and the `owui-lint` CLI are released as a matched pair, so a
 * single check covers both, and the release page hosts both artifacts.
 */
async function checkForUpdates(
  context: vscode.ExtensionContext,
): Promise<void> {
  const current = String(context.extension.packageJSON.version ?? "");

  let latest: string;
  try {
    const response = await fetch(RELEASES_API, {
      headers: {
        Accept: "application/vnd.github+json",
        "User-Agent": "owui-lint-vscode",
      },
    });
    if (!response.ok) {
      throw new Error(`GitHub API returned ${response.status}`);
    }
    const json = (await response.json()) as { tag_name?: string };
    const tag = json.tag_name ?? "";
    latest = tag.startsWith("v") ? tag.slice(1) : tag;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    void vscode.window.showErrorMessage(
      `owui-lint: failed to check for updates: ${message}`,
    );
    return;
  }

  if (!latest || !isNewerVersion(latest, current)) {
    void vscode.window.showInformationMessage(
      `owui-lint is up to date (v${current}).`,
    );
    return;
  }

  const open = "Open Release";
  const choice = await vscode.window.showInformationMessage(
    `owui-lint v${latest} is available (current: v${current}).`,
    open,
  );
  if (choice === open) {
    void vscode.env.openExternal(vscode.Uri.parse(RELEASES_PAGE));
  }
}

/** Returns true when `latest` is strictly newer than `current` (semver-ish). */
function isNewerVersion(latest: string, current: string): boolean {
  const toParts = (value: string): number[] =>
    value
      .split(".")
      .map((part) => Number.parseInt(part, 10))
      .map((n) => (Number.isNaN(n) ? 0 : n));

  const a = toParts(latest);
  const b = toParts(current);
  const length = Math.max(a.length, b.length);
  for (let i = 0; i < length; i += 1) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) {
      return diff > 0;
    }
  }
  return false;
}

/**
 * Updates the `owui-lint` CLI in place via its self-update subcommand.
 *
 * The server is stopped first because the running LSP process holds the
 * binary open, and the installer is run in a visible integrated terminal so
 * the user can see and consent to what it executes. The server is restarted
 * once the terminal closes.
 */
async function updateCli(): Promise<void> {
  const config = vscode.workspace.getConfiguration("owui-lint");
  const command = resolveExecutablePath(config.get<string>("path", "owui-lint"));

  await stopClient();

  const terminal = vscode.window.createTerminal("owui-lint update");
  terminal.show();
  terminal.sendText(`${quoteForShell(command)} update`);

  const listener = vscode.window.onDidCloseTerminal(async (closed) => {
    if (closed !== terminal) {
      return;
    }
    listener.dispose();
    if (isEnabled()) {
      await start();
    }
  });
}

/**
 * Quotes an executable path for the integrated terminal's shell.
 *
 * The terminal's default shell differs by platform: PowerShell on Windows,
 * a POSIX shell elsewhere. Their quoting rules are incompatible, so we branch
 * on `process.platform`. The fast path returns the value unquoted when it
 * contains only shell-safe characters (no quoting needed on any shell).
 */
function quoteForShell(value: string): string {
  if (/^[A-Za-z0-9._/-]+$/.test(value)) {
    return value;
  }

  if (process.platform === "win32") {
    // PowerShell: backslash is a literal path separator, not an escape
    // character, so POSIX-style `\"` escaping is wrong here. Inside a
    // double-quoted string an embedded `"` is escaped by doubling it (`""`).
    // A quoted string on its own is treated as a string literal, not a
    // command, so the call operator `&` is required to execute it.
    return `& "${value.replace(/"/g, '""')}"`;
  }

  // POSIX shells: escape the characters that stay special inside double quotes.
  return `"${value.replace(/(["\\$`])/g, "\\$1")}"`;
}

async function start(): Promise<void> {
  if (!isEnabled()) {
    return;
  }

  const next = createClient();
  client = next;

  try {
    await next.start();
  } catch (error) {
    if (client === next) {
      client = undefined;
    }
    const message = error instanceof Error ? error.message : String(error);
    void vscode.window.showErrorMessage(
      `owui-lint language server failed to start: ${message}`,
    );
  }
}

async function restart(): Promise<void> {
  if (restartInFlight) {
    return restartInFlight;
  }

  restartInFlight = (async () => {
    await stopClient();
    await start();
  })().finally(() => {
    restartInFlight = undefined;
  });

  return restartInFlight;
}

function resolveExecutablePath(configuredPath: string): string {
  const value = configuredPath.trim();
  if (value === "" || value === "owui-lint") {
    return "owui-lint";
  }

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const expanded = workspaceFolder
    ? value.replaceAll("${workspaceFolder}", workspaceFolder)
    : value;

  if (isAbsolutePath(expanded)) {
    return expanded;
  }

  if (workspaceFolder && looksLikeRelativePath(expanded)) {
    return vscode.Uri.joinPath(
      vscode.Uri.file(workspaceFolder),
      expanded.replace(/\\/g, "/"),
    ).fsPath;
  }

  return expanded;
}

function isAbsolutePath(value: string): boolean {
  return (
    value.startsWith("/") ||
    /^[A-Za-z]:[\\/]/.test(value) ||
    value.startsWith("\\\\")
  );
}

function looksLikeRelativePath(value: string): boolean {
  return value.startsWith(".") || value.includes("/") || value.includes("\\");
}

