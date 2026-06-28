import { useState, useCallback, useRef, useEffect } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { message as antdMessage } from 'antd';
import {
   sendChatMessage,
   chatOptions,
   commitChatHistory,
} from '../api/chat';
import type { ChatCitation } from '../types';

export interface ChatMessage {
   id: number;
   role: 'user' | 'assistant';
   content: string;
   mode?: ChatMode;
   citations?: ChatCitation[];
   createTime: number;
   status?: 'sending' | 'sent' | 'error';
}

export type ChatMode = 'default' | 'supplement';

export interface UseAiChatOptions {
   projectId: number;
   bidId: number;
   days?: number;
}

interface SupplementPair {
   userContent: string;
   aiContent: string;
}

interface PersistedAiChatState {
   messages: ChatMessage[];
   pendingSupplements: SupplementPair[];
   savedSupplementSignatures: string[];
}

const buildSupplementSignature = (pair: SupplementPair): string =>
   `${pair.userContent}\n---\n${pair.aiContent}`;

const collectSupplementPairsFromMessages = (
   messages: ChatMessage[],
   savedSignatures: Set<string>
): SupplementPair[] => {
   const pairs: SupplementPair[] = [];
   let pendingUser = '';
   for (const msg of messages) {
      if (msg.mode !== 'supplement') continue;
      if (msg.role === 'user') {
         pendingUser = String(msg.content || '').trim();
         continue;
      }
      if (msg.role === 'assistant') {
         const aiContent = String(msg.content || '').trim();
         if (!pendingUser || !aiContent) {
            pendingUser = '';
            continue;
         }
         const pair: SupplementPair = { userContent: pendingUser, aiContent };
         const signature = buildSupplementSignature(pair);
         if (!savedSignatures.has(signature)) {
            pairs.push(pair);
         }
         pendingUser = '';
      }
   }
   return pairs;
};

let _seed = 0;
const generateId = (): number => Number(`${Date.now()}_${++_seed}`);

const readPersistedAiChatState = (
   storageKey: string
): PersistedAiChatState | null => {
   if (typeof window === 'undefined') return null;
   try {
      const raw = window.localStorage.getItem(storageKey);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as Partial<PersistedAiChatState>;
      const messages = Array.isArray(parsed.messages) ? parsed.messages : [];
      const pendingSupplements = Array.isArray(parsed.pendingSupplements)
         ? parsed.pendingSupplements
         : [];
      const savedSupplementSignatures = Array.isArray(
         parsed.savedSupplementSignatures
      )
         ? parsed.savedSupplementSignatures
         : [];
      return {
         messages,
         pendingSupplements,
         savedSupplementSignatures,
      };
   } catch {
      return null;
   }
};

const writePersistedAiChatState = (
   storageKey: string,
   state: PersistedAiChatState
) => {
   if (typeof window === 'undefined') return;
   try {
      window.localStorage.setItem(storageKey, JSON.stringify(state));
   } catch {
      // ignore quota/storage errors
   }
};

export function useAiChat({ projectId, bidId, days = 10 }: UseAiChatOptions) {
   const queryClient = useQueryClient();
   const storageKey = `aiChat:${projectId}:${bidId}`;
   const initialPersistedState = useRef<PersistedAiChatState | null>(
      readPersistedAiChatState(storageKey)
   );
   const [messages, setMessages] = useState<ChatMessage[]>(
      () => initialPersistedState.current?.messages ?? []
   );
   const [pendingSupplements, setPendingSupplements] = useState<SupplementPair[]>(
      () => initialPersistedState.current?.pendingSupplements ?? []
   );
   const [savedSupplementSignatures, setSavedSupplementSignatures] = useState<Set<string>>(
      () =>
         new Set(initialPersistedState.current?.savedSupplementSignatures ?? [])
   );

   // 用于追踪正在发送的消息，以便在请求返回时更新状态
   const pendingMsgId = useRef<number | null>(null);
   const pendingMode = useRef<ChatMode>('default');
   const pendingUserContent = useRef<string>('');
   const latestSupplementUserContent = useRef<string>('');
   const latestSupplementAiContent = useRef<string>('');

   const historyQuery = useQuery(chatOptions.history(projectId, bidId, days));

   // 当历史记录请求成功返回时，初始化消息列表
   const isInitialized = useRef((initialPersistedState.current?.messages.length ?? 0) > 0);

   useEffect(() => {
      if (historyQuery.data && !isInitialized.current) {
         const mapped: ChatMessage[] = historyQuery.data.map((item) => ({
            id: item.id,
            role: item.role,
            content: item.content,
            citations: undefined,
            createTime: new Date(item.createTime).getTime(),
            status: 'sent' as const,
         }));
         queueMicrotask(() => {
            setMessages(mapped);
         });
         isInitialized.current = true;
      }
   }, [historyQuery.data]);

   useEffect(() => {
      writePersistedAiChatState(storageKey, {
         messages,
         pendingSupplements,
         savedSupplementSignatures: Array.from(savedSupplementSignatures),
      });
   }, [messages, pendingSupplements, savedSupplementSignatures, storageKey]);

   const sendMutation = useMutation({
      mutationFn: ({ content, mode }: { content: string; mode: ChatMode }) =>
         sendChatMessage({
            projectId,
            bidId,
            content,
            saveToKnowledgeBase: false,
            mode,
         }),

      onSuccess: (responseData) => {
         const aiMsg: ChatMessage = {
            id: generateId(),
            role: 'assistant',
            content: responseData.content,
            mode: pendingMode.current,
            citations: responseData.citations,
            createTime: Date.now(),
            status: 'sent',
         };
         if (pendingMode.current === 'supplement') {
            latestSupplementUserContent.current = pendingUserContent.current;
            latestSupplementAiContent.current = responseData.content || '';
            const userContent = latestSupplementUserContent.current.trim();
            const aiContent = latestSupplementAiContent.current.trim();
            if (userContent && aiContent) {
               setPendingSupplements((prev) => {
                  const base = Array.isArray(prev) ? prev : [];
                  return [...base, { userContent, aiContent }];
               });
            }
         }

         setMessages((prev) =>
            prev
               .map((m) =>
                  m.id === pendingMsgId.current ? { ...m, status: 'sent' as const } : m
               )
               .concat(aiMsg)
         );

         queryClient.invalidateQueries({ queryKey: ['auditResult'] });
         queryClient.invalidateQueries({ queryKey: ['auditStatus'] });
      },

      onError: (error: Error) => {
         antdMessage.error(`AI 请求失败: ${error.message}`);
         setMessages((prev) =>
            prev.map((m) =>
               m.id === pendingMsgId.current ? { ...m, status: 'error' as const } : m
            )
         );
      },
   });

   // ── 3. 暴露给组件的方法 ──
   const sendMessage = useCallback(
      (content: string, mode: ChatMode = 'default') => {
         const trimmed = content.trim();
         if (!trimmed || sendMutation.isPending) return;

         const userMsg: ChatMessage = {
            id: generateId(),
            role: 'user',
            content: trimmed,
            mode,
            createTime: Date.now(),
            status: 'sending',
         };
         pendingMsgId.current = userMsg.id;
         pendingMode.current = mode;
         pendingUserContent.current = trimmed;
         setMessages((prev) => [...prev, userMsg]);

         sendMutation.mutate({ content: trimmed, mode });
      },
      [sendMutation]
   );

   // 保存对话
   const saveMutation = useMutation({
      mutationFn: async (pairsToSave: SupplementPair[]) => {
         if (!Array.isArray(pairsToSave) || pairsToSave.length === 0) {
            throw new Error('暂无补充错误可保存');
         }
         const mergedUserContent = pairsToSave
            .map((pair, idx) => `【补充${idx + 1}】\n${pair.userContent}`)
            .join('\n\n');
         const mergedAiContent = pairsToSave
            .map((pair, idx) => `【补充${idx + 1}】\n${pair.aiContent}`)
            .join('\n\n');
         return commitChatHistory({
            projectId,
            bidId,
            useLatest: false,
            normalizeBeforeSave: true,
            userContent: mergedUserContent,
            aiContent: mergedAiContent,
         });
      },

      onSuccess: (normalizedSummary, pairsToSave) => {
         const savedCount = Array.isArray(pairsToSave) ? pairsToSave.length : 0;
         antdMessage.success(`已保存 ${savedCount} 条补充错误`);
         setSavedSupplementSignatures((prev) => {
            const next = new Set(prev);
            (pairsToSave || []).forEach((pair) => next.add(buildSupplementSignature(pair)));
            return next;
         });
         setPendingSupplements((prev) => {
            const remaining = (prev || []).filter(
               (pair) =>
                  !(pairsToSave || [])
                     .map((x) => buildSupplementSignature(x))
                     .includes(buildSupplementSignature(pair))
            );
            return remaining;
         });
         latestSupplementUserContent.current = '';
         latestSupplementAiContent.current = '';
         if (normalizedSummary?.trim()) {
            const summaryMsg: ChatMessage = {
               id: generateId(),
               role: 'assistant',
               content: `已保存记录，归纳内容如下：\n${normalizedSummary}`,
               createTime: Date.now(),
               status: 'sent',
            };
            setMessages((prev) => [...prev, summaryMsg]);
         }
         queryClient.invalidateQueries({
            queryKey: ['chatHistory', projectId, bidId],
         });
      },

      onError: (error: Error) => {
         antdMessage.error(`保存失败: ${error.message}`);
      },
   });

   const saveHistory = useCallback(() => {
      const queued = Array.isArray(pendingSupplements) ? pendingSupplements : [];
      const autoDetected = collectSupplementPairsFromMessages(messages, savedSupplementSignatures);
      const seen = new Set<string>();
      const pairsToSave = [...queued, ...autoDetected].filter((pair) => {
         const sign = buildSupplementSignature(pair);
         if (seen.has(sign)) return false;
         seen.add(sign);
         return true;
      });
      if (pairsToSave.length === 0) {
         antdMessage.warning('暂无补充错误可保存');
         return;
      }
      saveMutation.mutate(pairsToSave);
   }, [messages, pendingSupplements, savedSupplementSignatures, saveMutation]);

   const clearMessages = useCallback(() => {
      latestSupplementUserContent.current = '';
      latestSupplementAiContent.current = '';
      setPendingSupplements([]);
      setSavedSupplementSignatures(new Set());
      setMessages([]);
      if (typeof window !== 'undefined') {
         window.localStorage.removeItem(storageKey);
      }
   }, [storageKey]);

   return {
      messages,
      sendMessage,
      clearMessages,
      saveHistory,
      hasSavableSupplement:
         pendingSupplements.length > 0 ||
         collectSupplementPairsFromMessages(messages, savedSupplementSignatures).length > 0,
      savableSupplementCount: (() => {
         const autoDetected = collectSupplementPairsFromMessages(messages, savedSupplementSignatures);
         const signs = new Set<string>();
         [...pendingSupplements, ...autoDetected].forEach((pair) =>
            signs.add(buildSupplementSignature(pair))
         );
         return signs.size;
      })(),
      isSaving: saveMutation.isPending,
      isLoading: sendMutation.isPending,
      isHistoryLoading: historyQuery.isLoading,
   };
}
