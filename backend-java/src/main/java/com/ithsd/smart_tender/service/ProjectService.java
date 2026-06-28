package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.pojo.dto.ProjectDTO;
import com.ithsd.smart_tender.pojo.vo.ProjectVO;

import java.util.List;

public interface ProjectService {
    ProjectVO create(ProjectDTO projectDTO);

    void delete(Long id);

    ProjectVO update(ProjectDTO projectDTO);

    List<ProjectVO> listAll();

    List<ProjectVO> getMyProjects();

    boolean exists(String projectName);
}
