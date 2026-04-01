import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type CurrentConfig = {
  apiKey: string;
  baseUrl: string;
  authPath: string;
  configPath: string;
};

type AccountProfile = {
  id: string;
  name: string;
  apiKey: string;
  baseUrl: string;
  updatedAt: number;
};

type AppSnapshot = {
  current: CurrentConfig;
  profiles: AccountProfile[];
  profileStorePath: string;
  codexDirPath: string;
  platformLabel: string;
};

type SaveProfileResult = {
  profiles: AccountProfile[];
  savedId: string;
};

type StatusTone = "info" | "success" | "error";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root not found");
}

app.innerHTML = `
  <main class="shell">
    <section class="hero">
      <div>
        <p class="eyebrow">Tauri / Codex</p>
        <h1>账号变更器</h1>
        <p class="hero-copy">
          只改当前平台 Codex 配置目录中的两处真实配置：<code>auth.json</code> 的
          <code>OPENAI_API_KEY</code>，以及 <code>config.toml</code> 里
          <code>[model_providers.OpenAI]</code> 下的 <code>base_url</code>。macOS / Linux
          默认目录是 <code>~/.codex</code>，Windows 默认目录是
          <code>%USERPROFILE%\\.codex</code>。
        </p>
      </div>
      <div class="hero-note">
        <span class="chip chip--success">最小写入</span>
        <span class="chip">本地保存账号</span>
        <span class="chip">随时回读当前生效值</span>
      </div>
    </section>

    <section class="status-panel">
      <div id="status" class="status status--info" aria-live="polite">正在读取当前 Codex 配置…</div>
    </section>

    <section class="layout">
      <aside class="panel panel--sidebar">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Saved Accounts</p>
            <h2>已保存账号</h2>
          </div>
          <button id="refreshButton" class="button button--ghost" type="button">刷新</button>
        </div>

        <div class="storage-block">
          <span class="storage-label">当前平台</span>
          <code id="platformLabel">读取中…</code>
          <span class="storage-label">当前 Codex 目录</span>
          <code id="codexDirPath">读取中…</code>
          <span class="storage-label">账号存储文件</span>
          <code id="profileStorePath">读取中…</code>
        </div>

        <div id="emptyProfiles" class="empty-state" hidden>
          还没有保存的账号。右侧填好 API Key 和 base URL 后点“保存到列表”。
        </div>
        <div id="profileList" class="profile-list"></div>
      </aside>

      <section class="panel panel--main">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Live Config</p>
            <h2>当前生效配置</h2>
          </div>
          <button id="pullCurrentButton" class="button button--ghost" type="button">重新读取</button>
        </div>

        <div class="current-card">
          <div class="current-card__row">
            <span>API Key</span>
            <code id="currentApiKey">读取中…</code>
          </div>
          <div class="current-card__row">
            <span>base_url</span>
            <code id="currentBaseUrl">读取中…</code>
          </div>
          <div class="current-card__paths">
            <div>
              <span>auth.json</span>
              <code id="authPath">读取中…</code>
            </div>
            <div>
              <span>config.toml</span>
              <code id="configPath">读取中…</code>
            </div>
          </div>
        </div>

        <div class="panel-head panel-head--form">
          <div>
            <p class="panel-kicker">Editor</p>
            <h2>编辑与切换</h2>
          </div>
          <div class="toolbar">
            <button id="importCurrentButton" class="button button--ghost" type="button">导入当前配置</button>
            <button id="clearFormButton" class="button button--ghost" type="button">清空表单</button>
          </div>
        </div>

        <form id="profileForm" class="editor-grid">
          <label class="field">
            <span>账号名称</span>
            <input id="nameInput" name="name" type="text" placeholder="例如：主账号 / 备用账号 / 测试环境" />
          </label>

          <label class="field field--full">
            <span>OpenAI API Key</span>
            <input id="apiKeyInput" name="apiKey" type="text" spellcheck="false" autocomplete="off" placeholder="sk-..." />
          </label>

          <label class="field field--full">
            <span>OpenAI base_url</span>
            <input id="baseUrlInput" name="baseUrl" type="url" spellcheck="false" autocomplete="off" placeholder="http://localhost:8080" />
          </label>
        </form>

        <div class="action-row">
          <button id="saveButton" class="button button--secondary" type="button">保存到列表</button>
          <button id="applyButton" class="button button--primary" type="button">应用到 Codex</button>
        </div>
      </section>
    </section>
  </main>
`;

const statusEl = query<HTMLElement>("#status");
const currentApiKeyEl = query<HTMLElement>("#currentApiKey");
const currentBaseUrlEl = query<HTMLElement>("#currentBaseUrl");
const authPathEl = query<HTMLElement>("#authPath");
const configPathEl = query<HTMLElement>("#configPath");
const platformLabelEl = query<HTMLElement>("#platformLabel");
const codexDirPathEl = query<HTMLElement>("#codexDirPath");
const profileStorePathEl = query<HTMLElement>("#profileStorePath");
const profileListEl = query<HTMLElement>("#profileList");
const emptyProfilesEl = query<HTMLElement>("#emptyProfiles");

const nameInput = query<HTMLInputElement>("#nameInput");
const apiKeyInput = query<HTMLInputElement>("#apiKeyInput");
const baseUrlInput = query<HTMLInputElement>("#baseUrlInput");

const refreshButton = query<HTMLButtonElement>("#refreshButton");
const pullCurrentButton = query<HTMLButtonElement>("#pullCurrentButton");
const importCurrentButton = query<HTMLButtonElement>("#importCurrentButton");
const clearFormButton = query<HTMLButtonElement>("#clearFormButton");
const saveButton = query<HTMLButtonElement>("#saveButton");
const applyButton = query<HTMLButtonElement>("#applyButton");

const actionButtons = [
  refreshButton,
  pullCurrentButton,
  importCurrentButton,
  clearFormButton,
  saveButton,
  applyButton,
];

const state = {
  current: null as CurrentConfig | null,
  profiles: [] as AccountProfile[],
  selectedProfileId: "",
  pendingDeleteId: "",
};

refreshButton.addEventListener("click", () => {
  void refreshSnapshot("已刷新账号列表和当前配置。");
});

pullCurrentButton.addEventListener("click", () => {
  void refreshSnapshot("已重新读取当前 Codex 配置。");
});

importCurrentButton.addEventListener("click", () => {
  if (!state.current) {
    setStatus("当前配置还没读到，稍后再试。", "error");
    return;
  }

  if (!nameInput.value.trim()) {
    nameInput.value = "当前配置";
  }

  apiKeyInput.value = state.current.apiKey;
  baseUrlInput.value = state.current.baseUrl;
  setStatus("当前生效配置已导入表单。", "info");
});

clearFormButton.addEventListener("click", () => {
  state.selectedProfileId = "";
  state.pendingDeleteId = "";
  nameInput.value = "";
  apiKeyInput.value = "";
  baseUrlInput.value = "";
  renderProfiles();
  setStatus("表单已清空。", "info");
});

saveButton.addEventListener("click", () => {
  void saveProfile();
});

applyButton.addEventListener("click", () => {
  void applyForm();
});

profileListEl.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const actionNode = target.closest<HTMLElement>("[data-action]");

  if (actionNode) {
    const action = actionNode.dataset.action;
    const id = actionNode.dataset.id;

    if (!action || !id) {
      return;
    }

    if (action === "load") {
      state.pendingDeleteId = "";
      loadProfileIntoForm(id);
      return;
    }

    if (action === "apply") {
      state.pendingDeleteId = "";
      void applySavedProfile(id);
      return;
    }

    if (action === "delete") {
      if (state.pendingDeleteId !== id) {
        state.pendingDeleteId = id;
        renderProfiles();
        setStatus("再次点击“确认删除”以删除这个已保存账号。", "info");
        return;
      }

      void deleteProfile(id);
      return;
    }
  }

  const card = target.closest<HTMLElement>("[data-profile-id]");

  if (!card) {
    return;
  }

  const id = card.dataset.profileId;
  if (!id) {
    return;
  }

  state.selectedProfileId = id;
  state.pendingDeleteId = "";
  renderProfiles();
});

void refreshSnapshot();

async function refreshSnapshot(successMessage?: string) {
  setBusy(true);
  setStatus("正在读取当前 Codex 配置…", "info");

  try {
    const snapshot = await invoke<AppSnapshot>("load_snapshot");
    state.current = snapshot.current;
    state.profiles = snapshot.profiles;
    platformLabelEl.textContent = snapshot.platformLabel;
    codexDirPathEl.textContent = snapshot.codexDirPath;
    profileStorePathEl.textContent = snapshot.profileStorePath;

    if (!state.profiles.some((profile) => profile.id === state.selectedProfileId)) {
      state.selectedProfileId = "";
    }

    if (!state.profiles.some((profile) => profile.id === state.pendingDeleteId)) {
      state.pendingDeleteId = "";
    }

    renderCurrent();
    renderProfiles();
    setStatus(successMessage ?? "当前配置已读取。", "success");
  } catch (error) {
    setStatus(normalizeError(error), "error");
  } finally {
    setBusy(false);
  }
}

async function saveProfile() {
  const name = nameInput.value.trim();
  const apiKey = apiKeyInput.value.trim();
  const baseUrl = baseUrlInput.value.trim();

  if (!name) {
    setStatus("保存账号前先填写账号名称。", "error");
    return;
  }

  if (!apiKey || !baseUrl) {
    setStatus("保存账号前需要填写 API Key 和 base_url。", "error");
    return;
  }

  setBusy(true);
  setStatus("正在保存账号到本地列表…", "info");

  try {
    const result = await invoke<SaveProfileResult>("save_profile", {
      input: {
        id: state.selectedProfileId || null,
        name,
        apiKey,
        baseUrl,
      },
    });

    state.profiles = result.profiles;
    state.selectedProfileId = result.savedId;
    state.pendingDeleteId = "";
    renderProfiles();
    setStatus("账号已保存。", "success");
  } catch (error) {
    setStatus(normalizeError(error), "error");
  } finally {
    setBusy(false);
  }
}

async function applyForm() {
  const apiKey = apiKeyInput.value.trim();
  const baseUrl = baseUrlInput.value.trim();

  if (!apiKey || !baseUrl) {
    setStatus("应用前需要填写 API Key 和 base_url。", "error");
    return;
  }

  setBusy(true);
  setStatus("正在写入 Codex 配置…", "info");

  try {
    const current = await invoke<CurrentConfig>("apply_profile", {
      input: {
        apiKey,
        baseUrl,
      },
    });

    state.current = current;
    state.pendingDeleteId = "";
    renderCurrent();
    setStatus("切换成功，Codex 配置已更新。", "success");
  } catch (error) {
    setStatus(normalizeError(error), "error");
  } finally {
    setBusy(false);
  }
}

function loadProfileIntoForm(id: string) {
  const profile = state.profiles.find((item) => item.id === id);

  if (!profile) {
    setStatus("找不到要载入的账号。", "error");
    return;
  }

  state.selectedProfileId = id;
  state.pendingDeleteId = "";
  nameInput.value = profile.name;
  apiKeyInput.value = profile.apiKey;
  baseUrlInput.value = profile.baseUrl;
  renderProfiles();
  setStatus(`已载入账号：${profile.name}`, "info");
}

async function applySavedProfile(id: string) {
  const profile = state.profiles.find((item) => item.id === id);

  if (!profile) {
    setStatus("找不到要应用的账号。", "error");
    return;
  }

  state.selectedProfileId = id;
  nameInput.value = profile.name;
  apiKeyInput.value = profile.apiKey;
  baseUrlInput.value = profile.baseUrl;
  renderProfiles();
  await applyForm();
}

async function deleteProfile(id: string) {
  const profile = state.profiles.find((item) => item.id === id);

  if (!profile) {
    setStatus("找不到要删除的账号。", "error");
    return;
  }

  setBusy(true);
  setStatus("正在删除账号…", "info");

  try {
    const profiles = await invoke<AccountProfile[]>("delete_profile", { id });
    state.profiles = profiles;
    state.pendingDeleteId = "";

    if (state.selectedProfileId === id) {
      state.selectedProfileId = "";
    }

    renderProfiles();
    setStatus(`已删除账号：${profile.name}`, "success");
  } catch (error) {
    state.pendingDeleteId = "";
    renderProfiles();
    setStatus(normalizeError(error), "error");
  } finally {
    setBusy(false);
  }
}

function renderCurrent() {
  if (!state.current) {
    currentApiKeyEl.textContent = "未读取";
    currentBaseUrlEl.textContent = "未读取";
    authPathEl.textContent = "未读取";
    configPathEl.textContent = "未读取";
    return;
  }

  currentApiKeyEl.textContent = maskKey(state.current.apiKey);
  currentBaseUrlEl.textContent = state.current.baseUrl;
  authPathEl.textContent = state.current.authPath;
  configPathEl.textContent = state.current.configPath;
}

function renderProfiles() {
  const profiles = state.profiles;
  emptyProfilesEl.hidden = profiles.length > 0;

  profileListEl.innerHTML = profiles
    .map((profile) => {
      const activeClass = profile.id === state.selectedProfileId ? " profile-card--active" : "";
      const deleteLabel = profile.id === state.pendingDeleteId ? "确认删除" : "删除";

      return `
        <article class="profile-card${activeClass}" data-profile-id="${escapeHtml(profile.id)}">
          <div class="profile-card__head">
            <h3>${escapeHtml(profile.name)}</h3>
            <span>${formatTime(profile.updatedAt)}</span>
          </div>
          <p class="profile-card__meta">${escapeHtml(profile.baseUrl)}</p>
          <p class="profile-card__meta profile-card__meta--mono">${escapeHtml(maskKey(profile.apiKey))}</p>
          <div class="profile-card__actions">
            <button class="button button--tiny button--ghost" type="button" data-action="load" data-id="${escapeHtml(profile.id)}">载入</button>
            <button class="button button--tiny button--secondary" type="button" data-action="apply" data-id="${escapeHtml(profile.id)}">应用</button>
            <button class="button button--tiny button--danger" type="button" data-action="delete" data-id="${escapeHtml(profile.id)}">${deleteLabel}</button>
          </div>
        </article>
      `;
    })
    .join("");
}

function setBusy(isBusy: boolean) {
  for (const button of actionButtons) {
    button.disabled = isBusy;
  }

  const profileButtons = profileListEl.querySelectorAll<HTMLButtonElement>("button");
  for (const button of profileButtons) {
    button.disabled = isBusy;
  }
}

function setStatus(message: string, tone: StatusTone) {
  statusEl.className = `status status--${tone}`;
  statusEl.textContent = message;
}

function maskKey(value: string) {
  if (!value) {
    return "未设置";
  }

  if (value.length <= 12) {
    return value;
  }

  return `${value.slice(0, 8)}…${value.slice(-6)}`;
}

function formatTime(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

function normalizeError(error: unknown) {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "发生未知错误。";
}

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function query<T extends Element>(selector: string) {
  const node = document.querySelector<T>(selector);

  if (!node) {
    throw new Error(`Missing element: ${selector}`);
  }

  return node;
}
