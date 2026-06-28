package com.ithsd.smart_tender.service.engine;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.enums.AuditCheckTypeEnum;
import org.springframework.stereotype.Component;

import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;
import java.util.stream.Collectors;

@Component
public class AuditChainRunner {
    private final List<AuditCheck> checks;
    private final AuditProgressProperties progressProperties;
    private final Map<String, Integer> orderMap;

    public AuditChainRunner(List<AuditCheck> checks, AuditProgressProperties progressProperties) {
        this.checks = checks;
        this.progressProperties = progressProperties;
        this.orderMap = buildOrderMap();
    }

    public void run(AuditContext context, Consumer<AuditIssueEntity> issueConsumer) {
        List<AuditCheck> selectedChecks = checks.stream()
                .filter(check -> context.getEnabledChecks().contains(check.getCheckType()))
                .sorted(Comparator.comparingInt(check -> orderMap.getOrDefault(check.getCheckType(), Integer.MAX_VALUE)))
                .toList();

        for (AuditCheck check : selectedChecks) {
            context.setStage(check.getStage().name());
            try {
                List<AuditIssueEntity> addedIssues = context.addIssues(check.check(context));
                for (AuditIssueEntity issue : addedIssues) {
                    issueConsumer.accept(issue);
                }
            } catch (Exception ex) {
                context.addFailedStage(check.getStage().name());
            }
            context.increaseProgress(progressProperties.checkWeight(check.getCheckType()));
        }
    }

    private Map<String, Integer> buildOrderMap() {
        return List.of(AuditCheckTypeEnum.values()).stream()
                .collect(Collectors.toMap(AuditCheckTypeEnum::getValue, AuditCheckTypeEnum::ordinal, (a, b) -> a));
    }
}
