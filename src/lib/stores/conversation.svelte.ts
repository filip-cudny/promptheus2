import { SvelteMap } from "svelte/reactivity";
import { error as logError } from "@tauri-apps/plugin-log";
import { generateId } from "$lib/utils/id";
import {
  executeConversationTurn,
  generateConversationTitle,
  getSystemPrompt,
  releaseConversationContext,
  seedConversationContext,
  type ExecutionCallbacks,
} from "$lib/services/promptExecution";
import {
  addConversationEntry,
  updateConversationEntry,
  updateHistoryEntryTitle,
  getHistoryEntry,
} from "$lib/services/history";
import { getSkillBody } from "$lib/services/skills";
import {
  parseInputForSkills,
  composeSkillMessage,
  hasSkillReferences,
  type ResolvedSkillSegment,
} from "$lib/utils/skillComposer";
import { isSkillXml, extractSkillDisplayText } from "$lib/utils/skillDisplay";
import type {
  ConversationNode,
  ConversationImage,
  ConversationTree,
  MessagePair,
  TabState,
  ProcessedMessage,
  SerializedConversationNode,
  SerializedConversationTurn,
} from "$lib/types";

export function createEmptyTree(): ConversationTree {
  return {
    nodes: new SvelteMap(),
    root_node_id: null,
    current_path: [],
  };
}

export function createNode(
  role: "user" | "assistant",
  content: string,
  parentId: string | null,
  images: ConversationImage[] = [],
  textAttachments: string[] = [],
): ConversationNode {
  return {
    node_id: generateId(),
    parent_id: parentId,
    role,
    content,
    images,
    text_attachments: textAttachments,
    timestamp: new Date().toISOString(),
    children: [],
  };
}

export function getMessagePairs(tree: ConversationTree): MessagePair[] {
  const pairs: MessagePair[] = [];
  let messageNumber = 1;

  for (let i = 0; i < tree.current_path.length; i++) {
    const node = tree.nodes.get(tree.current_path[i]);
    if (!node || node.role !== "user") continue;

    const assistantId = tree.current_path[i + 1];
    const assistant = assistantId ? tree.nodes.get(assistantId) ?? null : null;

    pairs.push({ user: node, assistant, message_number: messageNumber++ });
  }

  return pairs;
}

export function getSiblings(
  tree: ConversationTree,
  nodeId: string,
): { siblings: string[]; index: number } {
  const node = tree.nodes.get(nodeId);
  if (!node || !node.parent_id) return { siblings: [nodeId], index: 0 };

  const parent = tree.nodes.get(node.parent_id);
  if (!parent) return { siblings: [nodeId], index: 0 };

  return {
    siblings: parent.children,
    index: parent.children.indexOf(nodeId),
  };
}

export function switchBranchInTree(
  tree: ConversationTree,
  nodeId: string,
  newChildIdx: number,
): void {
  const node = tree.nodes.get(nodeId);
  if (!node || !node.parent_id) return;

  const parent = tree.nodes.get(node.parent_id);
  if (!parent || newChildIdx < 0 || newChildIdx >= parent.children.length)
    return;

  const newNodeId = parent.children[newChildIdx];
  const parentIdx = tree.current_path.indexOf(node.parent_id);
  if (parentIdx === -1) return;

  tree.current_path = tree.current_path.slice(0, parentIdx + 1);
  tree.current_path.push(newNodeId);

  let currentId = newNodeId;
  while (true) {
    const current = tree.nodes.get(currentId);
    if (!current || current.children.length === 0) break;
    currentId = current.children[0];
    tree.current_path.push(currentId);
  }
}

function createTabState(tabName: string): TabState {
  return {
    tab_id: generateId(),
    tab_name: tabName,
    tree: createEmptyTree(),
    context_text: "",
    context_images: [],
    input_text: "",
    input_images: [],
    input_text_attachments: [],
    is_executing: false,
    is_streaming: false,
    streamed_content: "",
    execution_id: null,
    history_entry_id: null,
    pristine_input: null,
  };
}

function buildMessagesFromTree(
  tree: ConversationTree,
  contextImages: ConversationImage[],
): ProcessedMessage[] {
  const messages: ProcessedMessage[] = [];
  let isFirstUser = true;

  for (const nodeId of tree.current_path) {
    const node = tree.nodes.get(nodeId);
    if (!node) continue;

    if (node.role === "user") {
      const hasContextImages = isFirstUser && contextImages.length > 0;
      const hasNodeImages = node.images.length > 0;
      isFirstUser = false;

      let textContent = node.content;
      if (node.text_attachments.length > 0) {
        const wrappedAttachments = node.text_attachments
          .map((t, i) => `<pasted-text name="Text #${i + 1}">\n${t}\n</pasted-text>`)
          .join("\n\n");
        textContent = textContent
          ? `${wrappedAttachments}\n\n${textContent}`
          : wrappedAttachments;
      }

      if (hasContextImages || hasNodeImages) {
        const parts: Array<
          | { type: "text"; text: string }
          | { type: "image_url"; image_url: { url: string } }
        > = [];
        if (hasContextImages) {
          let contextIndex = 1;
          for (const img of contextImages) {
            parts.push({ type: "text", text: `[Context Image #${contextIndex++}]` });
            parts.push({
              type: "image_url",
              image_url: {
                url: `data:${img.media_type};base64,${img.data}`,
              },
            });
          }
        }
        let imageIndex = 1;
        for (const img of node.images) {
          parts.push({ type: "text", text: `[Image #${imageIndex++}]` });
          parts.push({
            type: "image_url",
            image_url: { url: `data:${img.media_type};base64,${img.data}` },
          });
        }
        parts.push({ type: "text", text: textContent });
        messages.push({ role: "user", content: parts });
      } else {
        messages.push({ role: "user", content: textContent });
      }
    } else {
      messages.push({ role: "assistant", content: node.content });
    }
  }

  return messages;
}

function serializeNodes(
  tree: ConversationTree,
): SerializedConversationNode[] {
  return Array.from(tree.nodes.values()).map((node) => ({
    node_id: node.node_id,
    parent_id: node.parent_id,
    role: node.role,
    content: node.content,
    image_paths: [],
    text_attachments: node.text_attachments,
    timestamp: node.timestamp,
    children: node.children,
  }));
}

function serializeTurns(pairs: MessagePair[]): SerializedConversationTurn[] {
  return pairs.map((pair) => ({
    turn_number: pair.message_number,
    message_text: isSkillXml(pair.user.content)
      ? extractSkillDisplayText(pair.user.content)
      : pair.user.content,
    message_image_paths: [],
    output_text: pair.assistant?.content ?? null,
    is_complete: pair.assistant !== null,
    output_versions: pair.assistant ? [pair.assistant.content] : [],
    current_version_index: 0,
  }));
}

function prependToLastUserMessage(
  messages: ProcessedMessage[],
  prefix: string,
): void {
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== "user") continue;

    if (typeof msg.content === "string") {
      messages[i] = { ...msg, content: `${prefix}\n\n${msg.content}` };
    } else if (Array.isArray(msg.content)) {
      const lastTextIdx = msg.content.findLastIndex(
        (p: { type: string }) => p.type === "text",
      );
      if (lastTextIdx !== -1) {
        const parts = [...msg.content];
        const part = parts[lastTextIdx] as { type: "text"; text: string };
        parts[lastTextIdx] = { type: "text", text: `${prefix}\n\n${part.text}` };
        messages[i] = { ...msg, content: parts };
      }
    }
    break;
  }
}

export function createConversationStore(
  promptId: string,
  promptName: string,
) {
  let tabs = $state<TabState[]>([createTabState("Chat 1")]);
  let activeTabId = $state(tabs[0].tab_id);

  const activeTab = $derived(
    tabs.find((t) => t.tab_id === activeTabId) ?? tabs[0],
  );
  const tree = $derived(activeTab.tree);
  const messagePairs = $derived(getMessagePairs(activeTab.tree));
  const isExecuting = $derived(activeTab.is_executing);
  const isStreaming = $derived(activeTab.is_streaming);
  const streamedContent = $derived(activeTab.streamed_content);
  const contextText = $derived(activeTab.context_text);
  const contextImages = $derived(activeTab.context_images);
  const inputText = $derived(activeTab.input_text);
  const inputImages = $derived(activeTab.input_images);
  const inputTextAttachments = $derived(activeTab.input_text_attachments);

  const canSend = $derived.by(() => {
    if (activeTab.is_executing) return false;
    if (
      !activeTab.input_text.trim() &&
      activeTab.input_images.length === 0 &&
      activeTab.input_text_attachments.length === 0
    )
      return false;
    const pairs = getMessagePairs(activeTab.tree);
    for (const pair of pairs) {
      if (!pair.user.content.trim() && pair.user.images.length === 0)
        return false;
      if (pair.assistant !== null && !pair.assistant.content.trim())
        return false;
    }
    return true;
  });

  const isRegenerateMode = $derived.by(() => {
    const path = activeTab.tree.current_path;
    if (path.length === 0) return false;
    const lastNode = activeTab.tree.nodes.get(path[path.length - 1]);
    if (!lastNode || lastNode.role !== "assistant") return false;
    return (
      !activeTab.input_text.trim() &&
      activeTab.input_images.length === 0 &&
      activeTab.input_text_attachments.length === 0
    );
  });

  function getTab(tabId: string): TabState | undefined {
    return tabs.find((t) => t.tab_id === tabId);
  }

  async function resolveSkillSegments(
    text: string,
  ): Promise<ResolvedSkillSegment[] | null> {
    if (!hasSkillReferences(text)) return null;

    const segments = parseInputForSkills(text);
    const hasAnySkill = segments.some((s) => s.skillName !== null);
    if (!hasAnySkill) return null;

    const resolved: ResolvedSkillSegment[] = [];
    for (const seg of segments) {
      if (seg.skillName) {
        try {
          const body = await getSkillBody(seg.skillName);
          resolved.push({
            skillName: seg.skillName,
            skillBody: body,
            input: seg.input,
          });
        } catch {
          resolved.push({
            skillName: seg.skillName,
            skillBody: "",
            input: seg.input,
          });
        }
      } else if (seg.input) {
        resolved.push({
          skillName: promptId,
          skillBody: "",
          input: seg.input,
        });
      }
    }

    return resolved.length > 0 ? resolved : null;
  }

  async function sendMessage(): Promise<{ success: boolean; result: string }> {
    const tab = getTab(activeTabId);
    if (!tab || tab.is_executing)
      return { success: false, result: "" };

    const text = tab.input_text.trim();
    const images = [...tab.input_images];
    const textAttachments = [...tab.input_text_attachments];
    if (!text && images.length === 0 && textAttachments.length === 0)
      return { success: false, result: "" };

    const skillSegments = await resolveSkillSegments(text);
    const storedContent = skillSegments
      ? composeSkillMessage(skillSegments)
      : text;

    const userNode = createNode(
      "user",
      storedContent,
      tab.tree.current_path.length > 0
        ? tab.tree.current_path[tab.tree.current_path.length - 1]
        : null,
      images,
      textAttachments,
    );
    tab.tree.nodes.set(userNode.node_id, userNode);

    if (userNode.parent_id) {
      const parent = tab.tree.nodes.get(userNode.parent_id);
      if (parent) parent.children.push(userNode.node_id);
    } else {
      tab.tree.root_node_id = userNode.node_id;
    }
    tab.tree.current_path.push(userNode.node_id);

    const assistantNode = createNode("assistant", "", userNode.node_id);
    tab.tree.nodes.set(assistantNode.node_id, assistantNode);
    userNode.children.push(assistantNode.node_id);
    tab.tree.current_path.push(assistantNode.node_id);

    tab.input_text = "";
    tab.input_images = [];
    tab.input_text_attachments = [];
    tab.is_executing = true;
    tab.is_streaming = true;
    tab.streamed_content = "";

    const isFirstMessage = getMessagePairs(tab.tree).length === 1;
    const hasDefaultTabName = /^Chat \d+$/.test(tab.tab_name);
    if (isFirstMessage && hasDefaultTabName && text) {
      generateConversationTitle(text)
        .then((title) => {
          if (title && hasDefaultTabName) {
            tab.tab_name = title;
            if (tab.history_entry_id) {
              updateHistoryEntryTitle(tab.history_entry_id, title).catch(() => {});
            }
          }
        })
        .catch(() => {});
    }

    let success = false;
    let resultText = "";

    try {
      const { messages: systemMessages, time_update } = await getSystemPrompt(
        tab.context_text || undefined,
        tab.tab_id,
      );
      const treeMessages = buildMessagesFromTree(tab.tree, tab.context_images);
      if (time_update) {
        prependToLastUserMessage(treeMessages, time_update);
      }
      const messages = [...systemMessages, ...treeMessages];

      const callbacks: ExecutionCallbacks = {
        onChunk: (_delta, accumulated) => {
          assistantNode.content = accumulated;
          tab.tree.nodes.set(assistantNode.node_id, assistantNode);
          tab.streamed_content = accumulated;
        },
        onDone: (fullText) => {
          assistantNode.content = fullText;
          tab.tree.nodes.set(assistantNode.node_id, assistantNode);
          tab.is_executing = false;
          tab.is_streaming = false;
          tab.streamed_content = "";
          success = true;
          resultText = fullText;
        },
        onError: (message) => {
          logError("Conversation execution error: " + message);
          assistantNode.content =
            assistantNode.content || `[error: ${message}]`;
          tab.tree.nodes.set(assistantNode.node_id, assistantNode);
          tab.is_executing = false;
          tab.is_streaming = false;
          tab.streamed_content = "";
        },
      };

      await executeConversationTurn(messages, callbacks, {
        promptId,
        promptName,
        skipClipboardCopy: true,
      });
    } catch (e) {
      logError("Failed to execute conversation turn: " + e);
      tab.is_executing = false;
      tab.is_streaming = false;
      tab.streamed_content = "";
    }

    if (success) {
      await saveToHistory();
    }

    return { success, result: resultText };
  }

  function stopExecution(): void {
    const tab = getTab(activeTabId);
    if (!tab) return;
    stopTabExecution(tab);
  }

  async function regenerate(nodeId: string): Promise<void> {
    const tab = getTab(activeTabId);
    if (!tab || tab.is_executing) return;

    const node = tab.tree.nodes.get(nodeId);
    if (!node || node.role !== "assistant" || !node.parent_id) return;

    const parentUser = tab.tree.nodes.get(node.parent_id);
    if (parentUser?.role === "user" && hasSkillReferences(parentUser.content)) {
      const segments = await resolveSkillSegments(parentUser.content);
      if (segments) {
        parentUser.content = composeSkillMessage(segments);
        tab.tree.nodes.set(parentUser.node_id, parentUser);
      }
    }

    const newAssistant = createNode("assistant", "", node.parent_id);
    tab.tree.nodes.set(newAssistant.node_id, newAssistant);

    const parent = tab.tree.nodes.get(node.parent_id);
    if (parent) parent.children.push(newAssistant.node_id);

    const parentIdx = tab.tree.current_path.indexOf(node.parent_id);
    if (parentIdx !== -1) {
      tab.tree.current_path = [
        ...tab.tree.current_path.slice(0, parentIdx + 1),
        newAssistant.node_id,
      ];
    }

    tab.is_executing = true;
    tab.is_streaming = true;
    tab.streamed_content = "";

    let regenerateSuccess = false;

    try {
      const { messages: systemMessages, time_update } = await getSystemPrompt(
        tab.context_text || undefined,
        tab.tab_id,
      );
      const treeMessages = buildMessagesFromTree(tab.tree, tab.context_images);
      if (time_update) {
        prependToLastUserMessage(treeMessages, time_update);
      }
      const messages = [...systemMessages, ...treeMessages];

      const callbacks: ExecutionCallbacks = {
        onChunk: (_delta, accumulated) => {
          newAssistant.content = accumulated;
          tab.tree.nodes.set(newAssistant.node_id, newAssistant);
          tab.streamed_content = accumulated;
        },
        onDone: (fullText) => {
          newAssistant.content = fullText;
          tab.tree.nodes.set(newAssistant.node_id, newAssistant);
          tab.is_executing = false;
          tab.is_streaming = false;
          tab.streamed_content = "";
          regenerateSuccess = true;
        },
        onError: (message) => {
          logError("Regeneration error: " + message);
          newAssistant.content =
            newAssistant.content || `[error: ${message}]`;
          tab.tree.nodes.set(newAssistant.node_id, newAssistant);
          tab.is_executing = false;
          tab.is_streaming = false;
          tab.streamed_content = "";
        },
      };

      await executeConversationTurn(messages, callbacks, {
        promptId,
        promptName,
        skipClipboardCopy: true,
      });
    } catch (e) {
      logError("Failed to regenerate: " + e);
      tab.is_executing = false;
      tab.is_streaming = false;
      tab.streamed_content = "";
    }

    if (regenerateSuccess) {
      await saveToHistory();
    }
  }

  function switchBranch(nodeId: string, direction: -1 | 1): void {
    const tab = getTab(activeTabId);
    if (!tab) return;

    const { siblings, index } = getSiblings(tab.tree, nodeId);
    const newIdx = index + direction;
    if (newIdx < 0 || newIdx >= siblings.length) return;

    switchBranchInTree(tab.tree, nodeId, newIdx);
  }

  function getBranchInfo(
    nodeId: string,
  ): { current: number; total: number } {
    const tab = getTab(activeTabId);
    if (!tab) return { current: 1, total: 1 };

    const { siblings, index } = getSiblings(tab.tree, nodeId);
    return { current: index + 1, total: siblings.length };
  }

  function isTabClean(tab: TabState): boolean {
    if (tab.tree.current_path.length > 0) return false;
    if (tab.input_text === "") return true;
    return tab.pristine_input !== null && tab.input_text === tab.pristine_input;
  }

  function skillInputPrefix(skillId: string): string {
    return skillId ? `/${skillId} ` : "";
  }

  function openForSkill(skillId: string, skillName: string): void {
    const tab = getTab(activeTabId);
    const prefix = skillInputPrefix(skillId);

    if (tab && isTabClean(tab)) {
      tab.input_text = prefix;
      tab.pristine_input = prefix || null;
      tab.tab_name = skillName || "Chat";
    } else {
      const newTab = createTabState(skillName || `Chat ${tabs.length + 1}`);
      newTab.input_text = prefix;
      newTab.pristine_input = prefix || null;
      tabs.push(newTab);
      activeTabId = newTab.tab_id;
    }
  }

  function setPristineInput(text: string): void {
    const tab = getTab(activeTabId);
    if (tab) tab.pristine_input = text || null;
  }

  function addTab(): void {
    const newTab = createTabState(`Chat ${tabs.length + 1}`);
    tabs.push(newTab);
    activeTabId = newTab.tab_id;
  }

  function stopTabExecution(tab: TabState): void {
    if (!tab.is_executing) return;

    const path = tab.tree.current_path;
    if (path.length > 0) {
      const lastNode = tab.tree.nodes.get(path[path.length - 1]);
      if (lastNode && lastNode.role === "assistant") {
        lastNode.content = (lastNode.content || "") + " [cancelled]";
      }
    }

    tab.is_executing = false;
    tab.is_streaming = false;
    tab.streamed_content = "";
  }

  function closeTab(tabId: string): void {
    const idx = tabs.findIndex((t) => t.tab_id === tabId);
    if (idx === -1) return;

    const closingTab = tabs[idx];
    stopTabExecution(closingTab);

    if (tabs.length <= 1) {
      tabs[0] = createTabState("Chat 1");
      activeTabId = tabs[0].tab_id;
      return;
    }

    tabs.splice(idx, 1);

    if (activeTabId === tabId) {
      activeTabId = tabs[Math.min(idx, tabs.length - 1)].tab_id;
    }
  }

  function switchTab(tabId: string): void {
    if (tabs.find((t) => t.tab_id === tabId)) {
      activeTabId = tabId;
    }
  }

  function updateNodeContent(nodeId: string, content: string): void {
    const tab = getTab(activeTabId);
    if (!tab) return;

    const node = tab.tree.nodes.get(nodeId);
    if (node) node.content = content;
  }

  function updateContextText(text: string): void {
    const tab = getTab(activeTabId);
    if (tab) tab.context_text = text;
  }

  function updateContextImages(images: ConversationImage[]): void {
    const tab = getTab(activeTabId);
    if (tab) tab.context_images = images;
  }

  function updateInputText(text: string): void {
    const tab = getTab(activeTabId);
    if (tab) tab.input_text = text;
  }

  function updateInputImages(images: ConversationImage[]): void {
    const tab = getTab(activeTabId);
    if (tab) tab.input_images = images;
  }

  function updateInputTextAttachments(attachments: string[]): void {
    const tab = getTab(activeTabId);
    if (tab) tab.input_text_attachments = attachments;
  }

  async function saveToHistory(): Promise<void> {
    const tab = getTab(activeTabId);
    if (!tab) return;

    const pairs = getMessagePairs(tab.tree);
    if (pairs.length === 0) return;

    const turns = serializeTurns(pairs);
    const nodes = serializeNodes(tab.tree);

    try {
      if (tab.history_entry_id) {
        await updateConversationEntry({
          entryId: tab.history_entry_id,
          turns,
          contextText: tab.context_text,
          contextImagePaths: [],
          nodes,
          rootNodeId: tab.tree.root_node_id,
          currentPath: tab.tree.current_path,
        });
      } else {
        const entryId = await addConversationEntry({
          turns,
          contextText: tab.context_text,
          contextImagePaths: [],
          promptId,
          promptName,
          success: true,
          error: null,
          nodes,
          rootNodeId: tab.tree.root_node_id,
          currentPath: tab.tree.current_path,
          tabId: tab.tab_id,
        });
        tab.history_entry_id = entryId;
        if (!/^Chat \d+$/.test(tab.tab_name)) {
          updateHistoryEntryTitle(entryId, tab.tab_name).catch(() => {});
        }
      }
    } catch (e) {
      logError("Failed to save conversation to history: " + e);
    }
  }

  async function restoreFromHistory(entryId: string, lastInteractionOnly?: boolean): Promise<void> {
    try {
      const existing = tabs.find((t) => t.history_entry_id === entryId);
      if (existing) {
        activeTabId = existing.tab_id;
        return;
      }

      const entry = await getHistoryEntry(entryId);
      if (!entry) return;

      const newTab = createTabState(
        entry.prompt_name ?? `Restored`,
      );
      newTab.history_entry_id = entryId;

      const restoredTree: ConversationTree = {
        nodes: new SvelteMap(),
        root_node_id: null,
        current_path: [],
      };

      if (entry.conversation_data && !lastInteractionOnly) {
        const data = entry.conversation_data;
        newTab.context_text = data.context_text;
        restoredTree.root_node_id = data.root_node_id;
        restoredTree.current_path = data.current_path;

        for (const serialized of data.nodes) {
          restoredTree.nodes.set(serialized.node_id, {
            node_id: serialized.node_id,
            parent_id: serialized.parent_id,
            role: serialized.role as "user" | "assistant",
            content: serialized.content,
            images: [],
            text_attachments: serialized.text_attachments ?? [],
            timestamp: serialized.timestamp,
            children: serialized.children,
          });
        }
      } else if (entry.input_content) {
        const userNodeId = `restored-user-${generateId()}`;
        const assistantNodeId = `restored-asst-${generateId()}`;
        const now = entry.timestamp;

        restoredTree.root_node_id = userNodeId;
        restoredTree.current_path = [userNodeId];

        restoredTree.nodes.set(userNodeId, {
          node_id: userNodeId,
          parent_id: null,
          role: "user",
          content: entry.input_content,
          images: [],
          text_attachments: [],
          timestamp: now,
          children: entry.output_content ? [assistantNodeId] : [],
        });

        if (entry.output_content) {
          restoredTree.current_path.push(assistantNodeId);
          restoredTree.nodes.set(assistantNodeId, {
            node_id: assistantNodeId,
            parent_id: userNodeId,
            role: "assistant",
            content: entry.output_content,
            images: [],
            text_attachments: [],
            timestamp: now,
            children: [],
          });
        }
      }

      newTab.tree = restoredTree;
      tabs.push(newTab);
      activeTabId = newTab.tab_id;

      if (entry.conversation_data?.resolved_context_section) {
        seedConversationContext(
          newTab.tab_id,
          entry.conversation_data.resolved_context_section,
        ).catch(() => {});
      }
    } catch (e) {
      logError("Failed to restore from history: " + e);
    }
  }

  function destroy(): void {
    for (const tab of tabs) {
      releaseConversationContext(tab.tab_id).catch(() => {});
    }
    tabs = [];
  }

  return {
    get tabs() {
      return tabs;
    },
    get activeTabId() {
      return activeTabId;
    },
    get tree() {
      return tree;
    },
    get messagePairs() {
      return messagePairs;
    },
    get isExecuting() {
      return isExecuting;
    },
    get isStreaming() {
      return isStreaming;
    },
    get streamedContent() {
      return streamedContent;
    },
    get canSend() {
      return canSend;
    },
    get isRegenerateMode() {
      return isRegenerateMode;
    },
    get contextText() {
      return contextText;
    },
    get contextImages() {
      return contextImages;
    },
    get inputText() {
      return inputText;
    },
    get inputImages() {
      return inputImages;
    },
    get inputTextAttachments() {
      return inputTextAttachments;
    },
    sendMessage,
    stopExecution,
    regenerate,
    switchBranch,
    getBranchInfo,
    openForSkill,
    setPristineInput,
    addTab,
    closeTab,
    switchTab,
    updateNodeContent,
    updateContextText,
    updateContextImages,
    updateInputText,
    updateInputImages,
    updateInputTextAttachments,
    saveToHistory,
    restoreFromHistory,
    destroy,
  };
}
