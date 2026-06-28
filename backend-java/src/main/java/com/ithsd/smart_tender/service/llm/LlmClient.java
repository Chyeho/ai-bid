package com.ithsd.smart_tender.service.llm;

public interface LlmClient {
    String complete(String checkType, String prompt);
}
