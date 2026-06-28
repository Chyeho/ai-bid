package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.pojo.dto.ChatRequestDTO;
import com.ithsd.smart_tender.pojo.result.Result;
import com.ithsd.smart_tender.pojo.vo.ChatMessageVO;
import com.ithsd.smart_tender.pojo.vo.ChatResponseVO;
import com.ithsd.smart_tender.service.ChatService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api/chat")
@RequiredArgsConstructor
public class ChatController {

    private final ChatService chatService;

    @PostMapping
    public Result<ChatResponseVO> chat(@RequestBody ChatRequestDTO requestDTO) {
        if (requestDTO.getProjectId() == null) {
            return Result.error(400, "项目ID不能为空");
        }
        if (requestDTO.getBidId() == null) {
            return Result.error(400, "标书ID不能为空");
        }
        if (requestDTO.getContent() == null || requestDTO.getContent().trim().isEmpty()) {
            return Result.error(400, "对话内容不能为空");
        }
        
        ChatResponseVO response = chatService.chat(requestDTO);
        return Result.success(response);
    }

    @GetMapping("/history")
    public Result<List<ChatMessageVO>> getHistory(
            @RequestParam Long projectId,
            @RequestParam Long bidId,
            @RequestParam(required = false) Integer days) {
        
        if (projectId == null) {
            return Result.error(400, "项目ID不能为空");
        }
        if (bidId == null) {
            return Result.error(400, "标书ID不能为空");
        }
        
        List<ChatMessageVO> history = chatService.getHistory(projectId, bidId, days);
        return Result.success(history);
    }

    @PostMapping("/commit")
    public Result<String> commit(@RequestBody com.ithsd.smart_tender.pojo.dto.ChatCommitRequestDTO req) {
        if (req.getProjectId() == null) {
            return Result.error(400, "项目ID不能为空");
        }
        if (req.getBidId() == null) {
            return Result.error(400, "标书ID不能为空");
        }
        String summary = chatService.commitKnowledge(req);
        return Result.success(summary);
    }
}
