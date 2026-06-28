package com.ithsd.smart_tender.config;

import io.milvus.param.IndexType;
import io.milvus.param.RpcStatus;
import io.milvus.param.index.CreateIndexParam;
import io.milvus.v2.common.DataType;
import io.milvus.v2.client.ConnectConfig;
import io.milvus.v2.client.MilvusClientV2;
import io.milvus.v2.common.IndexParam;
import io.milvus.v2.service.collection.request.AddFieldReq;
import io.milvus.v2.service.collection.request.CreateCollectionReq;
import io.milvus.v2.service.collection.request.HasCollectionReq;
import io.milvus.v2.service.index.request.CreateIndexReq;
import jakarta.annotation.PostConstruct;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import static com.amazonaws.AmazonServiceException.ErrorType.Client;

@Configuration
@ConditionalOnProperty(name = "milvus.enabled", havingValue = "true")
public class MilvusConfig {

    private static final Logger logger = LoggerFactory.getLogger(MilvusConfig.class);

    @Value("${milvus.host:localhost}")
    private String milvusHost;

    @Value("${milvus.port:19530}")
    private int milvusPort;

    @Value("${milvus.collection.name:knowledge_base}")
    private String collectionName;

    @Value("${milvus.dimension:1536}")
    private int dimension;

    @Bean
    public MilvusClientV2 milvusClient() {
        logger.info("正在连接 Milvus 服务：{}:{}", milvusHost, milvusPort);
        // 连接 Milvus 服务
        ConnectConfig connectConfig = ConnectConfig.builder()
                .uri(String.format("http://%s:%d", milvusHost, milvusPort))
                .build();
        
        MilvusClientV2 client = new MilvusClientV2(connectConfig);

        // 初始化集合
        initCollection(client);

        checkAndCreateIndex(client);

        return client;
    }

    private void initCollection(MilvusClientV2 client) {
        logger.info("检查集合是否存在：{}", collectionName);
        // 检查集合是否存在
        boolean collectionExists = client.hasCollection(HasCollectionReq.builder()
                .collectionName(collectionName)
                .build()
        );

        if (!collectionExists) {
            logger.info("创建集合：{}", collectionName);
            // 定义 Collection Schema
            CreateCollectionReq.CollectionSchema collectionSchema = client.createSchema();
            
            // 添加字段
            collectionSchema.addField(AddFieldReq.builder()
                    .fieldName("id")
                    .dataType(DataType.Int64)
                    .isPrimaryKey(Boolean.TRUE)
                    .autoID(Boolean.TRUE)
                    .description("主键 ID")
                    .build());
            
            collectionSchema.addField(AddFieldReq.builder()
                    .fieldName("embedding")
                    .dataType(DataType.FloatVector)
                    .dimension(dimension)
                    .description("向量嵌入")
                    .build());
            
            collectionSchema.addField(AddFieldReq.builder()
                    .fieldName("text")
                    .dataType(DataType.VarChar)
                    .maxLength(2000)
                    .description("文本内容")
                    .build());
            
            collectionSchema.addField(AddFieldReq.builder()
                    .fieldName("file_id")
                    .dataType(DataType.Int64)
                    .description("文件 ID")
                    .build());
            
            collectionSchema.addField(AddFieldReq.builder()
                    .fieldName("chunk_index")
                    .dataType(DataType.Int64)
                    .description("分块索引")
                    .build());
            
            collectionSchema.addField(AddFieldReq.builder()
                    .fieldName("metadata")
                    .dataType(DataType.JSON)
                    .description("元数据")
                    .build());

            // 创建集合
            client.createCollection(CreateCollectionReq.builder()
                    .collectionName(collectionName)
                    .collectionSchema(collectionSchema)
                    .build()
            );
            logger.info("集合创建完成：{}", collectionName);
        } else {
            logger.info("集合已存在：{}", collectionName);
        }
    }


    private void checkAndCreateIndex(MilvusClientV2 client) {
        try {
            // 尝试获取索引信息来判断是否存在
            var indexInfo = client.describeIndex(io.milvus.v2.service.index.request.DescribeIndexReq.builder()
                    .collectionName(collectionName)
                    .fieldName("embedding")
                    .build());

            if (indexInfo == null ) {
                logger.info("索引不存在，开始创建...");
                createVectorIndex(client);
            } else {
                logger.info("索引已存在：{}", indexInfo);
            }
        } catch (Exception e) {
            // 如果获取索引信息失败，说明索引不存在，尝试创建
            logger.info("未检测到索引，开始创建索引...");
            createVectorIndex(client);
        }
    }
    /**
     * 创建向量索引
     */
    private void createVectorIndex(MilvusClientV2 client) {
        try {
            logger.info("开始创建向量索引...");

            // 创建索引参数
            IndexParam indexParam = IndexParam.builder()
                    .fieldName("embedding")
                    .indexType(IndexParam.IndexType.FLAT)
                    .metricType(IndexParam.MetricType.COSINE)
                    .build();

            // 创建索引
            client.createIndex(CreateIndexReq.builder()
                    .collectionName(collectionName)
                    .indexParams(List.of(indexParam))
                    .build()
            );

            logger.info("✅ 向量索引创建成功");
        } catch (Exception e) {
            logger.error("❌ 向量索引创建失败：{}", e.getMessage(), e);
            throw new RuntimeException("向量索引创建失败：" + e.getMessage(), e);
        }
    }
}
