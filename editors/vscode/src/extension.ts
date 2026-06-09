import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let restartInFlight: Promise<void> | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("owui-lint.restartServer", () => restart()),
    vscode.commands.registerCommand("owui-lint.showOutput", () => {
      client?.outputChannel.show();
    }),
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("owui-lint.enable")) {
        if (isEnabled()) {
          await start();
        } else {
          const current = client;
          client = undefined;
          await current?.dispose();
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
  const current = client;
  client = undefined;
  if (current) {
    await current.dispose();
  }
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
    documentSelector: [{ scheme: "file", language: "python" }],
  };

  return new LanguageClient("owui-lint", "owui-lint", serverOptions, clientOptions);
}

function isEnabled(): boolean {
  return vscode.workspace
    .getConfiguration("owui-lint")
    .get<boolean>("enable", true);
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
    const current = client;
    client = undefined;
    if (current) {
      await current.dispose();
    }
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

