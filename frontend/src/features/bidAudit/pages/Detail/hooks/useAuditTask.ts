import { useState, useEffect, useCallback, useRef } from 'react';
import { useMutation } from '@tanstack/react-query';
import {
   createTask,
   connectStream,
   getAuditResult,
   getAuditStatus,
} from '../api/auditDetail';
import type {
  AuditIssue, AgentProgress, TraceEvent,
  PhaseEvent, StatsEvent,
  FindingAddedEvent, FindingUpdatedEvent, FindingRemovedEvent,
} from '@/types/audit';
import { ensureAuditIssue } from '../../../utils/mapFinding';

/** 连续轮询失败上限，超过则停止并清 stale 数据 */
const MAX_POLL_FAILURES = 5;

type StoredAuditTaskState = {
   taskId?: string;
   startedAt?: number;
};

export const useAuditTask = (bidId?: number) => {
   const [progress, setProgress] = useState(0);
   const [currentStage, setCurrentStage] = useState('准备开始...');
   const [issues, setIssues] = useState<AuditIssue[]>([]);
   const [isComplete, setIsComplete] = useState(false);
   const [error, setError] = useState<string | null>(null);
   const storageKey =
      typeof bidId === 'number' && !Number.isNaN(bidId)
         ? `auditTask:${bidId}`
         : null;
   const [taskId, setTaskId] = useState<string | null>(() => {
      if (!storageKey) return null;
      try {
         const raw = localStorage.getItem(storageKey);
         if (!raw) return null;
         const parsed = JSON.parse(raw) as StoredAuditTaskState;
         return parsed.taskId ?? null;
      } catch {
         return null;
      }
   });
   const [hydrated, setHydrated] = useState(false);
   const [shouldConnectStream, setShouldConnectStream] = useState(false);
   const [hasStartedAudit, setHasStartedAudit] = useState(false);
   const [lastStartAt, setLastStartAt] = useState(0);
   const [auditStartedAt, setAuditStartedAt] = useState<number>(() => {
      if (!storageKey) return 0;
      try {
         const raw = localStorage.getItem(storageKey);
         if (!raw) return 0;
         const parsed = JSON.parse(raw) as StoredAuditTaskState;
         const startedAt = Number(parsed.startedAt || 0);
         return Number.isFinite(startedAt) && startedAt > 0 ? startedAt : 0;
      } catch {
         return 0;
      }
   });
   const [elapsedSeconds, setElapsedSeconds] = useState(0);
   const [agentProgresses, setAgentProgresses] = useState<Map<string, AgentProgress>>(new Map());
   const [liveFeedEvents, setLiveFeedEvents] = useState<TraceEvent[]>([]);
   const [phaseEvent, setPhaseEvent] = useState<PhaseEvent | null>(null);
   const [phaseHistory, setPhaseHistory] = useState<PhaseEvent[]>([]);
   const [statsEvent, setStatsEvent] = useState<StatsEvent | null>(null);
   const [liveFindings, setLiveFindings] = useState<FindingAddedEvent[]>([]);
   const pollFailRef = useRef(0);
   const updateFinalElapsed = useCallback(() => {
      if (auditStartedAt <= 0) return;
      const finalSeconds = Math.max(0, Math.floor((Date.now() - auditStartedAt) / 1000));
      setElapsedSeconds((prev) => Math.max(prev, finalSeconds));
   }, [auditStartedAt]);

   const { mutate: startAudit, isPending: isStarting } = useMutation({
      mutationFn: (payload: { bidId: number; webSearchEnabled?: boolean; forceRefresh?: boolean }) =>
         createTask({
            bidId: payload.bidId,
            forceRefresh: !!payload.forceRefresh,
         }),
      onMutate: () => {
         const now = Date.now();
         setHasStartedAudit(true);
         setLastStartAt(now);
         setAuditStartedAt(now);
         setElapsedSeconds(0);
         setIsComplete(false);
         setIssues([]);
         setProgress(0);
         setError(null);
         setAgentProgresses(new Map());
         setLiveFeedEvents([]);
         setPhaseEvent(null);
         setPhaseHistory([]);
         setStatsEvent(null);
         setLiveFindings([]);
         setCurrentStage('正在创建审核任务...');
      },

      onSuccess: (data) => {
         if (!data?.taskId) {
            console.error('API 异常：后端未返回 taskId', data);
            setError('任务创建响应异常');
            return;
         }
         const startedAt = Date.now();
         if (storageKey) {
            try {
               localStorage.setItem(storageKey, JSON.stringify({ taskId: data.taskId, startedAt }));
            } catch (storageError) {
               console.error('[AuditTask] taskId 持久化失败:', storageError);
            }
         }
         setAuditStartedAt((prev) => prev || startedAt);
         setShouldConnectStream(true);
         setTaskId(data.taskId);
         setHydrated(true);
         setProgress(0);
         setIssues([]);
         setIsComplete(false);
         setError(null);
         setCurrentStage('任务已创建，等待流式数据...');
      },
      
      onError: (err: Error) => {
         console.error('[AuditTask] 任务创建失败:', err);
         setError(err.message || '任务创建失败');
         setCurrentStage('任务创建失败');
         setHasStartedAudit(false);
      },
   });

   useEffect(() => {
      let cancelled = false;

      const hydrate = async () => {
         const withinStartWindow =
            lastStartAt > 0 && Date.now() - lastStartAt < 30_000;
         if (withinStartWindow) {
            setHydrated(true);
            return;
         }
         if (isStarting) {
            setHydrated(true);
            return;
         }
         if (!taskId) {
            setHydrated(true);
            return;
         }
         if (shouldConnectStream) {
            setHydrated(true);
            return;
         }
         try {
            const status = await getAuditStatus(taskId);
            if (cancelled) return;
            const completed = status.status === 'completed';
            setIsComplete(completed);
            if (completed) {
               const result = await getAuditResult(taskId, { page: 1, size: 200 });
               if (cancelled) return;
               setIssues(result.issues || []);
               updateFinalElapsed();
               setProgress(100);
               setCurrentStage('审核完成');
               setShouldConnectStream(false);
               setHasStartedAudit(false);
            } else if (status.status === 'failed') {
               if (storageKey) {
                  localStorage.removeItem(storageKey);
               }
               setTaskId(null);
               setIssues([]);
               setProgress(0);
               setCurrentStage('审核失败');
               setIsComplete(false);
               setShouldConnectStream(false);
               setHasStartedAudit(false);
               setError('审核任务执行失败，请点击重新审核');
            } else {
               setAuditStartedAt((prev) => prev || Date.now());
               setProgress(status.progress || 0);
               setCurrentStage(status.stage || '审核进行中...');
               setIsComplete(false);
               setShouldConnectStream(true);
               setHasStartedAudit(true);
            }
            if (status.status !== 'failed') {
               setError(null);
            }
         } catch {
            // 清除 stale 数据，避免死循环
            if (storageKey) {
               try { localStorage.removeItem(storageKey); } catch { /* ignore */ }
            }
            setTaskId(null);
            setShouldConnectStream(false);
            setHasStartedAudit(false);
            setIsComplete(false);
            setError(null);
         } finally {
            if (!cancelled) setHydrated(true);
         }
      };

      setHydrated(false);
      hydrate();

      return () => {
         cancelled = true;
      };
   }, [taskId, storageKey, shouldConnectStream, isStarting, lastStartAt, updateFinalElapsed]);

   useEffect(() => {
      if (!taskId || isComplete || !hydrated || !shouldConnectStream) return;

      let isMounted = true;

      const lastEventId = (
         (typeof window !== 'undefined'
            && window.localStorage.getItem(`auditLastEvent:${taskId}`))
         || ''
      );
      connectStream(
         taskId,
         lastEventId,
         (type, payload) => {
            if (!isMounted) return;

            // 根据后端协议：event: issues / event: issue
            if (type === 'issues' || type === 'issue') {
               const items = Array.isArray(payload) ? payload : [payload];
               const mapped = items.map((item) =>
                  ensureAuditIssue(item)
               );
               setIssues((prev) => [...prev, ...mapped]);
            }
            // 根据后端协议：event: progress
            else if (type === 'progress') {
               const progressPayload = payload as {
                  progress?: number;
                  stage?: string;
               };
               setProgress(progressPayload.progress || 0);
               if (progressPayload.stage) setCurrentStage(progressPayload.stage);
            }
            // SSE §17.1: Agent 进度
            else if (type === 'agent_progress') {
               const ap = payload as AgentProgress;
               setAgentProgresses(prev => {
                  const next = new Map(prev);
                  next.set(ap.agent_id, ap);
                  return next;
               });
               setCurrentStage('Multi-Agent 并行审查中...');
            }
            // SSE §17.1: 实时动态
            else if (type === 'trace') {
               setLiveFeedEvents(prev => [...prev.slice(-100), payload as TraceEvent]);
            }
            // SSE §17.1: 管线阶段切换
            else if (type === 'phase') {
               const pe = payload as PhaseEvent;
               setPhaseEvent(pe);
               setPhaseHistory(prev => [...prev, pe]);
               setCurrentStage(pe.message);
            }
            // SSE §17.1: 阶段统计快照
            else if (type === 'stats') {
               setStatsEvent(payload as StatsEvent);
            }
            // SSE §17.1: 稳定后的风险发现
            else if (type === 'finding_added') {
               setLiveFindings(prev => [...prev, payload as FindingAddedEvent]);
            }
            // SSE §17.1: finding 被更新
            else if (type === 'finding_updated') {
               const fe = payload as FindingUpdatedEvent;
               setLiveFindings(prev => prev.map(f =>
                  f.risk_id === fe.risk_id
                     ? { ...f, ...fe.changes.reduce((acc, c) => ({ ...acc, [c.field]: c.new_value }), {}) as Partial<FindingAddedEvent> }
                     : f
               ));
            }
            // SSE §17.1: finding 被移除
            else if (type === 'finding_removed') {
               const fr = payload as FindingRemovedEvent;
               setLiveFindings(prev => prev.filter(f => f.risk_id !== fr.risk_id));
            }
         },
         // onComplete
         () => {
            if (!isMounted) return;
            setCurrentStage('审核流已结束，等待结果确认...');
         },
         // onError
         (err) => {
            if (!isMounted) return;
            console.error('[AuditTask] SSE 异常:', err);
            setError('实时数据连接中断，请刷新页面');
         }
      );

      return () => {
         isMounted = false;
      };
   }, [taskId, isComplete, hydrated, shouldConnectStream]);

   useEffect(() => {
      if (!taskId || isComplete || !shouldConnectStream) return;

      let stopped = false;

      const syncStatus = async () => {
         try {
            const status = await getAuditStatus(taskId);
            if (stopped) return;

            setProgress(status.progress || 0);
            if (status.stage) setCurrentStage(status.stage);

            if (status.status === 'completed') {
               const result = await getAuditResult(taskId, { page: 1, size: 200 });
               if (stopped) return;
               setIssues(result.issues || []);
               updateFinalElapsed();
               setIsComplete(true);
               setProgress(100);
               setCurrentStage('审核完成');
               setShouldConnectStream(false);
               setHasStartedAudit(false);
               setError(null);
               return;
            }
            setAuditStartedAt((prev) => prev || Date.now());

            const fallbackResult = await getAuditResult(taskId, {
               page: 1,
               size: 200,
            });
            if (stopped) return;
            const hasResult =
               (fallbackResult.issues?.length || 0) > 0 ||
               (status.status === 'completed' && !!fallbackResult.auditResult);
            if (hasResult) {
               setIssues(fallbackResult.issues || []);
               updateFinalElapsed();
               setIsComplete(true);
               setProgress(100);
               setCurrentStage('审核完成');
               setShouldConnectStream(false);
               setHasStartedAudit(false);
               setError(null);
               return;
            }

            if (status.status === 'failed') {
               setError('审核任务执行失败，请点击重新审核');
               setShouldConnectStream(false);
               setHasStartedAudit(false);
               if (storageKey) {
                  localStorage.removeItem(storageKey);
               }
               setTaskId(null);
            }
         } catch (e) {
            if (stopped) return;
            pollFailRef.current += 1;
            console.error(`[AuditTask] 状态轮询失败 (${pollFailRef.current}/${MAX_POLL_FAILURES}):`, e);
            if (pollFailRef.current >= MAX_POLL_FAILURES) {
               console.error('[AuditTask] 连续轮询失败达上限，停止轮询');
               setError('审核任务连接失败，请刷新页面后重试');
               setShouldConnectStream(false);
               setHasStartedAudit(false);
               if (storageKey) {
                  try { localStorage.removeItem(storageKey); } catch { /* ignore */ }
               }
               setTaskId(null);
            }
         }
      };

      pollFailRef.current = 0;
      syncStatus();
      const timer = window.setInterval(syncStatus, 3000);

      return () => {
         stopped = true;
         window.clearInterval(timer);
      };
   }, [taskId, isComplete, shouldConnectStream, updateFinalElapsed]);

   // 仅在审核流程活跃时计时；不再因为存在未完成 task 而持续计时（避免把对话时间算入）
   const isAuditingNow = hasStartedAudit || isStarting || shouldConnectStream;

   useEffect(() => {
      if (!isAuditingNow) return;
      if (!auditStartedAt) {
         const now = Date.now();
         setAuditStartedAt(now);
         setElapsedSeconds(0);
         return;
      }
      const syncElapsed = () => {
         setElapsedSeconds(Math.max(0, Math.floor((Date.now() - auditStartedAt) / 1000)));
      };
      syncElapsed();
      const timer = window.setInterval(syncElapsed, 1000);
      return () => {
         window.clearInterval(timer);
      };
   }, [isAuditingNow, auditStartedAt]);

   return {
      taskId,
      startAudit,
      isStarting,
      isAuditing: isAuditingNow,
      progress,
      currentStage,
      elapsedSeconds,
      issues,
      isComplete,
      error,
      summary: {
         totalIssues: issues.length,
         high: issues.filter((i) => i?.severity === 'high').length,
         medium: issues.filter((i) => i?.severity === 'medium').length,
         low: issues.filter((i) => i?.severity === 'low').length,
         info: issues.filter((i) => i?.severity === 'info').length,
      },
      agentProgresses,
      liveFeedEvents,
      phaseEvent,
      phaseHistory,
      statsEvent,
      liveFindings,
   };
};
