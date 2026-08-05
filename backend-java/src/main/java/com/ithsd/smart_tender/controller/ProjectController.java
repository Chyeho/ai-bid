package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.dto.ProjectDTO;
import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.ProjectVO;
import com.ithsd.smart_tender.service.ProjectService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api/projects")
@RequiredArgsConstructor
public class ProjectController {

    private final ProjectService projectService;

    @PostMapping
    public Result<ProjectVO> create(@RequestBody ProjectDTO projectDTO) {
        if (projectDTO.getProjectName() == null || projectDTO.getProjectName().trim().isEmpty()) {
            return Result.error(400, "项目名称不能为空");
        }
        // 项目名称不能和已有的项目重复
        if (projectService.exists(projectDTO.getProjectName())) {
            return Result.error(400, "项目已存在");
        }
        ProjectVO vo = projectService.create(projectDTO);
        return Result.success(vo);
    }

    @DeleteMapping("/{id}")
    public Result delete(@PathVariable Long id) {
        projectService.delete(id);
        return Result.success();
    }

    @PutMapping
    public Result<ProjectVO> update(@RequestBody ProjectDTO projectDTO) {
        if (projectDTO.getId() == null) {
            return Result.error(400, "项目ID不能为空");
        }
        ProjectVO vo = projectService.update(projectDTO);
        if (vo == null) {
            return Result.error(404, "项目不存在");
        }
        return Result.success(vo);
    }

    /**
     * 查询所有项目，包含标书等详细信息
     * @return
     */
    @GetMapping
    public Result<List<ProjectVO>> listAll() {
        List<ProjectVO> list = projectService.listAll();
        return Result.success(list);
    }

    /**
     * 查询本人创建的所有项目，不包含标书等详细信息
     * @return 项目列表
     */
    @GetMapping("/my")
    public Result<List<ProjectVO>> getMyProjects() {
        List<ProjectVO> list = projectService.getMyProjects();
        return Result.success(list);
    }
}
