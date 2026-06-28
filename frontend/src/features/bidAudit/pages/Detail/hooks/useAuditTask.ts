import { useState, useEffect, useCallback } from 'react';
import { useMutation } from '@tanstack/react-query';
import {
   createTask,
   connectStream,
   getAuditResult,
   getAuditStatus,
} from '../api/auditDetail';
import type { AuditIssue } from '../types';

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
   const updateFinalElapsed = useCallback(() => {
      if (auditStartedAt <= 0) return;
      const finalSeconds = Math.max(0, Math.floor((Date.now() - auditStartedAt) / 1000));
      setElapsedSeconds((prev) => Math.max(prev, finalSeconds));
   }, [auditStartedAt]);

   const { mutate: startAudit, isPending: isStarting } = useMutation({
      mutationFn: (payload: { bidId: number; webSearchEnabled?: boolean; forceRefresh?: boolean }) =>
         createTask({
            bidId: payload.bidId,
            enabledChecks: ['budget', 'legal', 'demand'],
            webSearchEnabled: !!payload.webSearchEnabled,
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
            setError('历史审核恢复失败，可重新点击开始审核');
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

      connectStream(
         taskId,
         '',
         (type, payload) => {
            if (!isMounted) return;

            // 根据后端协议：event: issues
            if (type === 'issues') {
               setIssues((prev) => [...prev, payload as AuditIssue]);
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
            console.error('[AuditTask] 状态轮询失败:', e);
         }
      };

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
         critical: issues.filter((i) => i?.severity === 'critical').length,
         warning: issues.filter((i) => i?.severity === 'warning').length,
         info: issues.filter((i) => i?.severity === 'info').length,
      },
   };
};
