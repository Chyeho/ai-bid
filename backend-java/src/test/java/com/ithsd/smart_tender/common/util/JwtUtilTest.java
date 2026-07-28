package com.ithsd.smart_tender.common.util;

import io.jsonwebtoken.Claims;
import io.jsonwebtoken.ExpiredJwtException;
import io.jsonwebtoken.MalformedJwtException;
import io.jsonwebtoken.SignatureException;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

import java.util.Date;
import java.util.HashMap;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("JwtUtil — JWT 令牌工具类单元测试")
class JwtUtilTest {

    private static final String SECRET_KEY = "test-secret-key-for-unit-testing-must-be-long-enough";
    private static final long VALID_TTL = 3600_000L; // 1 hour
    private static final Long USER_ID = 42L;
    private static final String USERNAME = "testuser";

    private Map<String, Object> createClaims(Long userId, String username) {
        Map<String, Object> claims = new HashMap<>();
        claims.put("userId", userId);
        claims.put("username", username);
        return claims;
    }

    @Test
    @DisplayName("createJWT: 应返回非空令牌字符串")
    void createToken_shouldReturnNonNullString() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims);
        assertNotNull(token);
        assertFalse(token.isEmpty());
    }

    @Test
    @DisplayName("createJWT: 不同载荷生成不同令牌")
    void createToken_withDifferentClaims_shouldGenerateDifferentTokens() {
        Map<String, Object> claims1 = createClaims(1L, "userA");
        Map<String, Object> claims2 = createClaims(2L, "userB");
        String token1 = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims1);
        String token2 = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims2);
        assertNotEquals(token1, token2);
    }

    @Test
    @DisplayName("parseJWT: 应提取正确载荷")
    void parseToken_shouldExtractCorrectClaims() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims);

        Claims parsed = JwtUtil.parseJWT(SECRET_KEY, token);

        assertNotNull(parsed);
        assertEquals(USER_ID, ((Number) parsed.get("userId")).longValue());
        assertEquals(USERNAME, parsed.get("username"));
    }

    @Test
    @DisplayName("parseJWT: 应包含过期时间")
    void parseToken_shouldContainExpiration() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims);

        Claims parsed = JwtUtil.parseJWT(SECRET_KEY, token);

        Date expiration = parsed.getExpiration();
        assertNotNull(expiration);
        assertTrue(expiration.after(new Date(System.currentTimeMillis() - 1000)));
    }

    @Test
    @DisplayName("getUserIdFromToken: 应从有效令牌中提取用户 ID")
    void getUserIdFromToken_shouldReturnUserId() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims);

        Claims parsed = JwtUtil.parseJWT(SECRET_KEY, token);
        Long extractedUserId = ((Number) parsed.get("userId")).longValue();

        assertEquals(USER_ID, extractedUserId);
    }

    @Test
    @DisplayName("validateToken: 有效令牌应返回 true")
    void validateToken_withValidToken_shouldReturnTrue() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims);

        boolean valid = true;
        try {
            JwtUtil.parseJWT(SECRET_KEY, token);
        } catch (Exception e) {
            valid = false;
        }
        assertTrue(valid);
    }

    @Test
    @DisplayName("validateToken: 过期令牌应返回 false")
    void validateToken_withExpiredToken_shouldReturnFalse() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        // Use negative TTL so the token is already expired
        String token = JwtUtil.createJWT(SECRET_KEY, -5000L, claims);

        boolean valid = true;
        try {
            JwtUtil.parseJWT(SECRET_KEY, token);
        } catch (ExpiredJwtException e) {
            valid = false;
        }
        assertFalse(valid);
    }

    @Test
    @DisplayName("validateToken: 无效签名令牌应返回 false")
    void validateToken_withWrongSignature_shouldReturnFalse() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT("different-secret-key-not-matching-the-original", VALID_TTL, claims);

        boolean valid = true;
        try {
            JwtUtil.parseJWT(SECRET_KEY, token);
        } catch (SignatureException e) {
            valid = false;
        }
        assertFalse(valid);
    }

    @Test
    @DisplayName("parseJWT: 过期令牌应抛出 ExpiredJwtException")
    void parseToken_withExpiredToken_shouldThrowExpiredJwtException() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, -5000L, claims);

        assertThrows(ExpiredJwtException.class,
                () -> JwtUtil.parseJWT(SECRET_KEY, token));
    }

    @Test
    @DisplayName("parseJWT: 篡改令牌应抛出 MalformedJwtException 或 SignatureException")
    void parseToken_withTamperedToken_shouldThrowException() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT(SECRET_KEY, VALID_TTL, claims);
        // Tamper: modify the payload segment
        String tampered = token.substring(0, token.lastIndexOf('.') - 1) + ".tampered";

        assertThrows(Exception.class,
                () -> JwtUtil.parseJWT(SECRET_KEY, tampered));
    }

    @ParameterizedTest
    @NullAndEmptySource
    @ValueSource(strings = {"not.a.token", "abc.def.ghi", "eyJhbGciOiJIUzI1NiJ9.notjson.sig"})
    @DisplayName("parseJWT: 格式错误/空/Null 令牌应抛出异常")
    void parseToken_withInvalidToken_shouldThrowException(String invalidToken) {
        assertThrows(Exception.class,
                () -> JwtUtil.parseJWT(SECRET_KEY, invalidToken));
    }

    @Test
    @DisplayName("parseJWT: 用错误的密钥解析应抛出 SignatureException")
    void parseToken_withWrongSecret_shouldThrowSignatureException() {
        Map<String, Object> claims = createClaims(USER_ID, USERNAME);
        String token = JwtUtil.createJWT("a-completely-different-secret-key", VALID_TTL, claims);

        assertThrows(SignatureException.class,
                () -> JwtUtil.parseJWT(SECRET_KEY, token));
    }
}
