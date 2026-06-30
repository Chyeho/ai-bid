package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.model.dto.TenderDTO;
import com.ithsd.smart_tender.model.dto.TenderPageQueryDTO;
import com.ithsd.smart_tender.model.result.PageResult;
import com.ithsd.smart_tender.model.vo.TenderProjectVO;
import com.ithsd.smart_tender.model.vo.TenderStatsVO;
import com.ithsd.smart_tender.model.vo.TenderVO;
import org.springframework.web.multipart.MultipartFile;

import java.util.List;

public interface TenderService {
    TenderVO upload(MultipartFile file, TenderDTO tenderDTO);

    PageResult page(TenderPageQueryDTO tenderPageQueryDTO);
    
    TenderStatsVO getStats(TenderPageQueryDTO tenderPageQueryDTO);

    TenderVO getById(Long id);

    /**
     * 获取指定项目下的所有标书版本列表
     * @param projectId 项目ID
     * @return 标书版本列表
     */
    List<TenderVO> getVersionsByProjectId(Long projectId);
    
    /**
     * 获取所有项目列表（聚合后的视图）
     * @return 项目列表
     */
    List<TenderProjectVO> getProjects();

    void delete(Long id);

    // 获取某个用户的所有上传标书id
    List<Long> getBidIdsByUserId(Long userId);
}
