package com.ithsd.smart_tender.service.llm;

import org.springframework.stereotype.Component;

@Component
public class StubLlmClient implements LlmClient {
    @Override
    public String complete(String checkType, String prompt) {
        String category = checkType;
        if (category == null || category.isBlank()) {
            category = "budget";
        }
        return "{\"severity\":\"warning\",\"category\":\"" + category + "\",\"description\":\"" + category + " 维度疑点\",\"suggestion\":\"请人工复核该维度关键条款\",\"reference\":\"stub_llm_v1\",\"location\":{\"pageNumber\":1,\"sectionName\":\"" + category + "章节\",\"context\":\"由 stub 生成的示例问题\"}}\n"
                + "{\"severity\":\"info\",\"category\":\"" + category + "\",\"description\":\"" + category + " 维度提示\",\"suggestion\":\"可进一步补充佐证材料\",\"reference\":\"stub_llm_v1\"}";
    }
}
