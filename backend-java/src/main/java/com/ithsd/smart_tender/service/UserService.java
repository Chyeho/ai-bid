package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.pojo.dto.UserLoginDTO;
import com.ithsd.smart_tender.pojo.dto.UserRegisterDTO;
import com.ithsd.smart_tender.pojo.entity.User;

public interface UserService {
    User login(UserLoginDTO userLoginDTO);

    void register(UserRegisterDTO userRegisterDTO);
}
