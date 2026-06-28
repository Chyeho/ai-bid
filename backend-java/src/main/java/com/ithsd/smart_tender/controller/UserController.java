package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.pojo.dto.UserLoginDTO;
import com.ithsd.smart_tender.pojo.dto.UserRegisterDTO;
import com.ithsd.smart_tender.pojo.entity.User;
import com.ithsd.smart_tender.pojo.result.Result;
import com.ithsd.smart_tender.pojo.vo.UserInfoVO;
import com.ithsd.smart_tender.pojo.vo.UserLoginVO;
import com.ithsd.smart_tender.service.UserService;
import com.ithsd.smart_tender.utils.JwtUtil;
import io.jsonwebtoken.Claims;
import lombok.RequiredArgsConstructor;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.util.StringUtils;
import org.springframework.web.bind.annotation.*;

import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.TimeUnit;

@RestController
@RequestMapping("/api/auth")
@RequiredArgsConstructor
public class UserController {

    private final UserService userService;
    private final StringRedisTemplate stringRedisTemplate;

    private static final String SECRET_KEY = "smart_tender_secret_key_123456";
    private static final long EXPIRATION_TIME = 1000 * 60 * 60 * 24; // 24 hours

    @PostMapping("/login")
    public Result<UserLoginVO> login(@RequestBody UserLoginDTO userLoginDTO) {
        User user = null;
        System.out.println(userLoginDTO);
        try {
            user = userService.login(userLoginDTO);
        } catch (Exception e) {
            return Result.error(401, e.getMessage());
        }


        Map<String, Object> claims = new HashMap<>();
        claims.put("userId", user.getId());
        claims.put("username", user.getUsername());

        String token = JwtUtil.createJWT(SECRET_KEY, EXPIRATION_TIME, claims);

        stringRedisTemplate.opsForValue().set("login:token:" + token, user.getId().toString(), EXPIRATION_TIME, TimeUnit.MILLISECONDS);

        UserInfoVO userInfoVO = UserInfoVO.builder()
                .id(user.getId())
                .username(user.getUsername())
                .realName(user.getRealName())
                .build();

        UserLoginVO vo = UserLoginVO.builder()
                .token(token)
                .userInfo(userInfoVO)
                .build();

        return Result.success(vo);
    }

    @PostMapping("/logout")
    public Result logout(@RequestHeader("Authorization") String token) {
        if (StringUtils.hasText(token) && token.startsWith("Bearer ")) {
            token = token.substring(7);
            stringRedisTemplate.delete("login:token:" + token);
        }
        return Result.success();
    }

    @PostMapping("/refresh")
    public Result refresh(@RequestHeader("Authorization") String token) {
        if (!StringUtils.hasText(token) || !token.startsWith("Bearer ")) {
            return Result.error(401, "Invalid token format");
        }
        token = token.substring(7);


        try {
            Claims claims = JwtUtil.parseJWT(SECRET_KEY, token);
            String userId = stringRedisTemplate.opsForValue().get("login:token:" + token);
            
            if (userId == null) {
                return Result.error(401, "Token invalid or expired");
            }

            String newToken = JwtUtil.createJWT(SECRET_KEY, EXPIRATION_TIME, claims);

            stringRedisTemplate.delete("login:token:" + token);
            stringRedisTemplate.opsForValue().set("login:token:" + newToken, userId, EXPIRATION_TIME, TimeUnit.MILLISECONDS);

            Map<String, String> data = new HashMap<>();
            data.put("token", newToken);
            
            return Result.success(data);

        } catch (Exception e) {
            return Result.error(401, "Token validation failed: " + e.getMessage());
        }
    }

    @PostMapping("/register")
    public Result register(@RequestBody UserRegisterDTO userRegisterDTO) {
        userService.register(userRegisterDTO);
        return Result.success();
    }
}
