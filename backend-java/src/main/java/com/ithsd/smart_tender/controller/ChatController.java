package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.dto.ChatRequestDTO;
import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.ChatMessageVO;
import com.ithsd.smart_tender.model.vo.ChatResponseVO;
import com.ithsd.smart_tender.service.ChatService;
import lombok.RequiredArgsConstructor;
import org.springframework.http.MediaType;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter;

import java.io.IOException;
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

    /**
     * SSE 流式对话。
     *
     * <p>POST /api/chat/stream，返回 text/event-stream。
     * 前端通过 fetch + ReadableStream 读取 thinking / tool_call / answer / done 事件。</p>
     */
    @PostMapping(value = "/stream", produces = MediaType.TEXT_EVENT_STREAM_VALUE)
    public SseEmitter chatStream(@RequestBody ChatRequestDTO requestDTO) {
        if (requestDTO.getProjectId() == null) {
            SseEmitter err = new SseEmitter();
            try {
                err.send(SseEmitter.event().name("error").data("{\"message\":\"项目ID不能为空\"}"));
                err.complete();
            } catch (IOException e) {
                err.completeWithError(e);
            }
            return err;
        }
        if (requestDTO.getBidId() == null) {
            SseEmitter err = new SseEmitter();
            try {
                err.send(SseEmitter.event().name("error").data("{\"message\":\"标书ID不能为空\"}"));
                err.complete();
            } catch (IOException e) {
                err.completeWithError(e);
            }
            return err;
        }
        if (requestDTO.getContent() == null || requestDTO.getContent().trim().isEmpty()) {
            SseEmitter err = new SseEmitter();
            try {
                err.send(SseEmitter.event().name("error").data("{\"message\":\"对话内容不能为空\"}"));
                err.complete();
            } catch (IOException e) {
                err.completeWithError(e);
            }
            return err;
        }
        return chatService.chatStream(requestDTO);
    }

}
