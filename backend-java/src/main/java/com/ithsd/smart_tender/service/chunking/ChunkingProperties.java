package com.ithsd.smart_tender.service.chunking;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Component
@ConfigurationProperties(prefix = "audit.chunk")
public class ChunkingProperties {
    private int targetLength = 1000;
    private int minLength = 800;
    private int maxLength = 1200;
    private double overlapRatio = 0.20d;
    private double overlapMinRatio = 0.15d;
    private double overlapMaxRatio = 0.25d;
    private String splitUnit = "paragraph";
    private boolean titleInjection = true;
    private boolean denoiseEnabled = true;
    private String mode = "hybrid";
    private double semanticSimilarityThreshold = 0.72d;
    private int semanticMinLength = 320;
    private int semanticMaxLength = 1800;
    private String strategyVersion = "chunk-v1";
    private String stableIdVersion = "stable-id-v1";

    public int getTargetLength() {
        return targetLength;
    }

    public void setTargetLength(int targetLength) {
        this.targetLength = targetLength;
    }

    public int getMinLength() {
        return minLength;
    }

    public void setMinLength(int minLength) {
        this.minLength = minLength;
    }

    public int getMaxLength() {
        return maxLength;
    }

    public void setMaxLength(int maxLength) {
        this.maxLength = maxLength;
    }

    public double getOverlapRatio() {
        return overlapRatio;
    }

    public void setOverlapRatio(double overlapRatio) {
        this.overlapRatio = overlapRatio;
    }

    public double getOverlapMinRatio() {
        return overlapMinRatio;
    }

    public void setOverlapMinRatio(double overlapMinRatio) {
        this.overlapMinRatio = overlapMinRatio;
    }

    public double getOverlapMaxRatio() {
        return overlapMaxRatio;
    }

    public void setOverlapMaxRatio(double overlapMaxRatio) {
        this.overlapMaxRatio = overlapMaxRatio;
    }

    public String getSplitUnit() {
        return splitUnit;
    }

    public void setSplitUnit(String splitUnit) {
        this.splitUnit = splitUnit;
    }

    public boolean isTitleInjection() {
        return titleInjection;
    }

    public void setTitleInjection(boolean titleInjection) {
        this.titleInjection = titleInjection;
    }

    public boolean isDenoiseEnabled() {
        return denoiseEnabled;
    }

    public void setDenoiseEnabled(boolean denoiseEnabled) {
        this.denoiseEnabled = denoiseEnabled;
    }

    public String getMode() {
        return mode;
    }

    public void setMode(String mode) {
        this.mode = mode;
    }

    public double getSemanticSimilarityThreshold() {
        return semanticSimilarityThreshold;
    }

    public void setSemanticSimilarityThreshold(double semanticSimilarityThreshold) {
        this.semanticSimilarityThreshold = semanticSimilarityThreshold;
    }

    public int getSemanticMinLength() {
        return semanticMinLength;
    }

    public void setSemanticMinLength(int semanticMinLength) {
        this.semanticMinLength = semanticMinLength;
    }

    public int getSemanticMaxLength() {
        return semanticMaxLength;
    }

    public void setSemanticMaxLength(int semanticMaxLength) {
        this.semanticMaxLength = semanticMaxLength;
    }

    public String getStrategyVersion() {
        return strategyVersion;
    }

    public void setStrategyVersion(String strategyVersion) {
        this.strategyVersion = strategyVersion;
    }

    public String getStableIdVersion() {
        return stableIdVersion;
    }

    public void setStableIdVersion(String stableIdVersion) {
        this.stableIdVersion = stableIdVersion;
    }
}
