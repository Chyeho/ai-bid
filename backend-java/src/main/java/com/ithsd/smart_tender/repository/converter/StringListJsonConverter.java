package com.ithsd.smart_tender.repository.converter;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.persistence.AttributeConverter;
import jakarta.persistence.Converter;

import java.util.ArrayList;
import java.util.List;

@Converter
public class StringListJsonConverter implements AttributeConverter<List<String>, String> {
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();
    private static final TypeReference<List<String>> TYPE_REFERENCE = new TypeReference<>() {
    };

    @Override
    public String convertToDatabaseColumn(List<String> attribute) {
        try {
            if (attribute == null) {
                return null;
            }
            return OBJECT_MAPPER.writeValueAsString(attribute);
        } catch (JsonProcessingException e) {
            throw new IllegalArgumentException("invalid list json", e);
        }
    }

    @Override
    public List<String> convertToEntityAttribute(String dbData) {
        try {
            if (dbData == null || dbData.isBlank()) {
                return new ArrayList<>();
            }
            JsonNode node = OBJECT_MAPPER.readTree(dbData);
            if (node.isArray()) {
                return OBJECT_MAPPER.convertValue(node, TYPE_REFERENCE);
            }
            if (node.isTextual()) {
                String text = node.asText();
                if (text == null || text.isBlank()) {
                    return new ArrayList<>();
                }
                return OBJECT_MAPPER.readValue(text, TYPE_REFERENCE);
            }
            return new ArrayList<>();
        } catch (JsonProcessingException e) {
            throw new IllegalArgumentException("invalid db json", e);
        }
    }
}
