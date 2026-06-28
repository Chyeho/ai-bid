package com.ithsd.smart_tender.service.trigger;

import com.ithsd.smart_tender.pojo.entity.RagTriggerOutboxEntity;

public interface RagTriggerHttpClient {
    TriggerHttpResult postTrigger(RagTriggerOutboxEntity entity);

    record TriggerHttpResult(int statusCode, String body, boolean success, boolean retryable, String errorMessage) {
    }
}
