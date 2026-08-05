package com.ithsd.smart_tender.sse;

import com.ithsd.smart_tender.model.enums.SseEventTypeEnum;
import io.micrometer.core.instrument.simple.SimpleMeterRegistry;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.springframework.web.servlet.mvc.method.annotation.ResponseBodyEmitter;
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter;

import java.lang.reflect.Field;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Consumer;

import static org.assertj.core.api.Assertions.assertThat;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

/**
 * Unit tests for {@link SseHub}.
 * <p>
 * Covers subscribe, emit, close, connection counting, emitter callback cleanup,
 * and concurrent safety. Callbacks registered on emitters cannot be triggered
 * via {@link SseEmitter#complete()} in a plain unit test (the servlet container
 * {@code Handler} is absent), so reflection is used to invoke the
 * {@link ResponseBodyEmitter#completionCallback completionCallback} and
 * {@link ResponseBodyEmitter#errorCallback errorCallback} delegates directly.
 */
@ExtendWith(MockitoExtension.class)
class SseHubTest {

    @Mock
    private RedisSseConnectionStateStore redisStore;

    /** Factory helper -- uses a real SimpleMeterRegistry so Micrometer counters/gauges work. */
    private SseHub createHub() {
        return new SseHub(new SimpleMeterRegistry(), redisStore);
    }

    // ---------------------------------------------------------------
    // subscribe
    // ---------------------------------------------------------------

    @Test
    void subscribe_createsNewEmitterAndIncrementsCount() {
        SseHub hub = createHub();

        SseEmitter emitter = hub.subscribe("task-1");

        assertThat(emitter).isNotNull();
        assertThat(hub.totalConnections()).isEqualTo(1);
        verify(redisStore).upsert("task-1", 1, "connected");
    }

    @Test
    void subscribe_withExistingTaskId_addsMultipleEmitters() {
        SseHub hub = createHub();

        hub.subscribe("shared-task");
        hub.subscribe("shared-task");

        assertThat(hub.totalConnections()).isEqualTo(2);
        verify(redisStore, times(2)).upsert(eq("shared-task"), anyInt(), eq("connected"));
    }

    @Test
    void subscribe_multipleTasks_tracksSeparately() {
        SseHub hub = createHub();

        hub.subscribe("task-a");
        hub.subscribe("task-b");
        hub.subscribe("task-a");

        assertThat(hub.totalConnections()).isEqualTo(3);
        assertThat(hub.taskConnections("task-a")).isEqualTo(2);
        assertThat(hub.taskConnections("task-b")).isEqualTo(1);
        assertThat(hub.taskConnections("nonexistent")).isEqualTo(0);
    }

    // ---------------------------------------------------------------
    // emit
    // ---------------------------------------------------------------

    @Test
    void emit_sendsDataAndRefreshesTtl() {
        SseHub hub = createHub();
        hub.subscribe("task-1");

        // Emit a progress event -- send should succeed and TTL be refreshed
        hub.emit("task-1", SseEventTypeEnum.PROGRESS, "{\"progress\":42}");

        assertThat(hub.totalConnections()).isEqualTo(1);
        verify(redisStore).refreshTtl("task-1");
    }

    @Test
    void emit_toNonExistentTaskId_doesNothing() {
        SseHub hub = createHub();

        hub.emit("no-such-task", SseEventTypeEnum.PROGRESS, "data");

        assertThat(hub.totalConnections()).isEqualTo(0);
        verify(redisStore, never()).refreshTtl(anyString());
    }

    @Test
    void emit_afterClosingTask_handlesGracefully() {
        SseHub hub = createHub();
        hub.subscribe("task-1");
        hub.close("task-1");

        // Emit to a closed task should find no emitters
        hub.emit("task-1", SseEventTypeEnum.PROGRESS, "data");

        assertThat(hub.totalConnections()).isEqualTo(0);
    }

    @Test
    void emit_withEventId_sendsSuccessfully() {
        SseHub hub = createHub();
        hub.subscribe("task-1");

        hub.emit("task-1", SseEventTypeEnum.PROGRESS, "data", "evt-001");

        assertThat(hub.totalConnections()).isEqualTo(1);
        verify(redisStore).refreshTtl("task-1");
    }

    // ---------------------------------------------------------------
    // close
    // ---------------------------------------------------------------

    @Test
    void close_removesAllEmittersForTask() {
        SseHub hub = createHub();
        hub.subscribe("task-1");
        hub.subscribe("task-1");

        hub.close("task-1");

        assertThat(hub.totalConnections()).isEqualTo(0);
    }

    @Test
    void close_otherTasksNotAffected() {
        SseHub hub = createHub();
        hub.subscribe("task-a");
        hub.subscribe("task-b");
        hub.subscribe("task-a");

        hub.close("task-a");

        assertThat(hub.taskConnections("task-a")).isZero();
        assertThat(hub.taskConnections("task-b")).isEqualTo(1);
        assertThat(hub.totalConnections()).isEqualTo(1);
    }

    @Test
    void close_nonExistentTaskId_doesNothing() {
        SseHub hub = createHub();

        hub.close("non-existent");

        assertThat(hub.totalConnections()).isEqualTo(0);
    }

    // ---------------------------------------------------------------
    // getActiveConnections / totalConnections
    // ---------------------------------------------------------------

    @Test
    void totalConnections_returnsZero_whenEmpty() {
        SseHub hub = createHub();
        assertThat(hub.totalConnections()).isZero();
    }

    @Test
    void totalConnections_aggregatesAcrossTasks() {
        SseHub hub = createHub();
        hub.subscribe("t1");
        hub.subscribe("t2");
        hub.subscribe("t1");
        hub.subscribe("t3");

        assertThat(hub.totalConnections()).isEqualTo(4);
    }

    // ---------------------------------------------------------------
    // Emitter lifecycle callbacks (onCompletion / onError / onTimeout)
    //
    // SseEmitter stores callbacks via setDelegate but only fires them
    // through its servlet-container Handler.  In a unit test the
    // Handler is null so complete() / completeWithError() are no-ops
    // for callback invocation.  We use reflection to grab and invoke
    // the delegates directly, which exercises the exact same lambdas
    // that `remove(taskId, emitterId)` wires at subscribe time.
    // ---------------------------------------------------------------

    @Test
    void emitter_onCompletion_removesEmitter() throws Exception {
        SseHub hub = createHub();
        SseEmitter emitter = hub.subscribe("task-1");
        assertThat(hub.totalConnections()).isEqualTo(1);

        // Invoke the completionCallback delegate directly
        callbackDelegate(emitter, "completionCallback", Runnable.class).run();

        assertThat(hub.totalConnections()).isEqualTo(0);
    }

    @Test
    void emitter_onCompletion_onlyRemovesCompletedEmitter() throws Exception {
        SseHub hub = createHub();
        SseEmitter e1 = hub.subscribe("shared-task");
        SseEmitter e2 = hub.subscribe("shared-task");
        assertThat(hub.totalConnections()).isEqualTo(2);

        // Invoke completionCallback on only the first emitter
        callbackDelegate(e1, "completionCallback", Runnable.class).run();

        assertThat(hub.totalConnections()).isEqualTo(1);
        assertThat(hub.taskConnections("shared-task")).isEqualTo(1);
    }

    @Test
    void emitter_onError_removesEmitter() throws Exception {
        SseHub hub = createHub();
        SseEmitter emitter = hub.subscribe("task-1");
        assertThat(hub.totalConnections()).isEqualTo(1);

        // Invoke the errorCallback delegate directly
        callbackDelegate(emitter, "errorCallback", Consumer.class)
                .accept(new RuntimeException("simulated SSE error"));

        assertThat(hub.totalConnections()).isEqualTo(0);
    }

    @Test
    void emitter_onTimeout_usesSameRemoveLogic() throws Exception {
        // The onTimeout, onCompletion, and onError callbacks all register
        // the identical lambda `() -> remove(taskId, emitterId)`.
        // Here we verify the timeout path is wired the same way.
        SseHub hub = createHub();
        SseEmitter emitter = hub.subscribe("task-1");
        assertThat(hub.totalConnections()).isEqualTo(1);

        callbackDelegate(emitter, "timeoutCallback", Runnable.class).run();

        assertThat(hub.totalConnections()).isEqualTo(0);
    }

    /** Reflectively extract and return the delegate from one of the three
     *  {@link ResponseBodyEmitter} callback wrappers. */
    @SuppressWarnings("unchecked")
    private static <T> T callbackDelegate(SseEmitter emitter, String callbackFieldName,
                                           Class<T> delegateType) throws Exception {
        Field callbackField = ResponseBodyEmitter.class.getDeclaredField(callbackFieldName);
        callbackField.setAccessible(true);
        Object callback = callbackField.get(emitter);

        Field delegateField = callback.getClass().getDeclaredField("delegate");
        delegateField.setAccessible(true);
        return (T) delegateField.get(callback);
    }

    // ---------------------------------------------------------------
    // Concurrent safety
    // ---------------------------------------------------------------

    @Test
    void concurrentSubscribe_noDataLoss() throws InterruptedException {
        SseHub hub = createHub();
        int threadCount = 8;
        int subscribesPerThread = 20;
        CountDownLatch latch = new CountDownLatch(threadCount);
        AtomicInteger exceptions = new AtomicInteger(0);

        ExecutorService executor = Executors.newFixedThreadPool(threadCount);
        for (int i = 0; i < threadCount; i++) {
            String taskId = "con-task-" + i;
            executor.submit(() -> {
                try {
                    for (int j = 0; j < subscribesPerThread; j++) {
                        hub.subscribe(taskId);
                    }
                } catch (Exception e) {
                    exceptions.incrementAndGet();
                } finally {
                    latch.countDown();
                }
            });
        }
        latch.await();
        executor.shutdown();

        assertThat(exceptions).hasValue(0);
        assertThat(hub.totalConnections()).isEqualTo(threadCount * subscribesPerThread);
    }

    @Test
    void concurrentSubscribeAndEmit_noException() throws InterruptedException {
        SseHub hub = createHub();
        int threadCount = 4;
        int iterations = 30;
        CountDownLatch latch = new CountDownLatch(threadCount);
        AtomicInteger exceptions = new AtomicInteger(0);

        ExecutorService executor = Executors.newFixedThreadPool(threadCount);
        for (int i = 0; i < threadCount; i++) {
            String taskId = "emit-task-" + i;
            executor.submit(() -> {
                try {
                    for (int j = 0; j < iterations; j++) {
                        hub.subscribe(taskId);
                        hub.emit(taskId, SseEventTypeEnum.PROGRESS, "payload-" + j);
                    }
                } catch (Exception e) {
                    exceptions.incrementAndGet();
                } finally {
                    latch.countDown();
                }
            });
        }
        latch.await();
        executor.shutdown();

        assertThat(exceptions).hasValue(0);
        assertThat(hub.totalConnections()).isEqualTo(threadCount * iterations);
    }

    @Test
    void concurrentSubscribeAndClose_noDataCorruption() throws InterruptedException {
        SseHub hub = createHub();
        int threadCount = 6;
        int iterations = 15;
        CountDownLatch latch = new CountDownLatch(threadCount);
        AtomicInteger exceptions = new AtomicInteger(0);

        ExecutorService executor = Executors.newFixedThreadPool(threadCount);
        for (int i = 0; i < threadCount; i++) {
            String taskId = "close-task-" + i;
            executor.submit(() -> {
                try {
                    for (int j = 0; j < iterations; j++) {
                        hub.subscribe(taskId);
                        hub.emit(taskId, SseEventTypeEnum.TRACE, "msg-" + j);
                    }
                    hub.close(taskId);
                } catch (Exception e) {
                    exceptions.incrementAndGet();
                } finally {
                    latch.countDown();
                }
            });
        }
        latch.await();
        executor.shutdown();

        assertThat(exceptions).hasValue(0);
        // All emitters should have been removed by close
        assertThat(hub.totalConnections()).isEqualTo(0);
    }
}
