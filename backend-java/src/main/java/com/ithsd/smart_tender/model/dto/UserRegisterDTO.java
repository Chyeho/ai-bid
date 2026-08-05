package com.ithsd.smart_tender.model.dto;

import lombok.Data;
import java.io.Serializable;

@Data
public class UserRegisterDTO implements Serializable {
    private String username;
    private String password;
    private String realName;
    private String email;
    private String phone;
}
