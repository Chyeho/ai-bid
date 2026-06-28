package com.ithsd.smart_tender.sse;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.ithsd.smart_tender.pojo.entity.AuditTaskEventEntity;
import com.ithsd.smart_tender.pojo.enums.SseEventTypeEnum;
import com.ithsd.smart_tender.repository.AuditTaskEventRepository;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.data.domain.PageRequest;
import org.springframework.stereotype.Service;
import org.springframework.util.StringUtils;

import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;

@Service
public class AuditTaskEventService {
    private static final Logger log = LoggerFactory.getLogger(AuditTaskEventService.class);
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();
    private final AuditTaskEventRepository eventRepository;
    private final AuditSseProperties sseProperties;

    public AuditTaskEventService(AuditTaskEventRepository eventRepository, AuditSseProperties sseProperties) {
        this.eventRepository = eventRepository;
        this.sseProperties = sseProperties;
    }

    public String persist(String taskId, SseEventTypeEnum eventType, Object payload) {
        if (!StringUtils.hasText(taskId) || eventType == null || payload == null) {
            return null;
        }
        try {
            AuditTaskEventEntity event = new AuditTaskEventEntity();
            event.setTaskId(taskId);
            event.setEventType(eventType.getEventName());
            event.setEventData(OBJECT_MAPPER.writeValueAsString(payload));
            event.setCreatedAt(LocalDateTime.now());
            AuditTaskEventEntity saved = eventRepository.save(event);
            return String.valueOf(saved.getId());
        } catch (RuntimeException | JsonProcessingException ex) {
            log.warn("persist task event failed, taskId={}, eventType={}", taskId, eventType.getEventName(), ex);
            return null;
        }
    }

    public List<ReplaySseEvent> replay(String taskId, String lastEventId) {
        List<ReplaySseEvent> events = new ArrayList<>();
        if (!StringUtils.hasText(taskId)) {
            return events;
        }
        long startId = parseLastEventId(lastEventId);
        int limit = Math.max(1, sseProperties.getReplayMaxEvents());
        List<AuditTaskEventEntity> entities = eventRepository.findByTaskIdAndIdGreaterThanOrderByIdAsc(taskId, startId, PageRequest.of(0, limit));
        for (AuditTaskEventEntity entity : entities) {
            SseEventTypeEnum eventType = SseEventTypeEnum.fromEventName(entity.getEventType());
            if (eventType == null) {
                continue;
            }
            try {
                ReplaySseEvent replayEvent = new ReplaySseEvent();
                replayEvent.setEventId(String.valueOf(entity.getId()));
                replayEvent.setEventType(eventType);
                replayEvent.setData(OBJECT_MAPPER.readTree(entity.getEventData()));
                events.add(replayEvent);
            } catch (JsonProcessingException ex) {
                log.warn("parse replay event failed, eventId={}", entity.getId(), ex);
            }
        }
        return events;
    }

    private long parseLastEventId(String lastEventId) {
        if (!StringUtils.hasText(lastEventId)) {
            return 0L;
        }
        try {
            return Long.parseLong(lastEventId);
        } catch (NumberFormatException ex) {
            return 0L;
        }
    }
}
