package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.model.dto.UserLoginDTO;
import com.ithsd.smart_tender.model.dto.UserRegisterDTO;
import com.ithsd.smart_tender.model.entity.User;

public interface UserService {
    User login(UserLoginDTO userLoginDTO);

    void register(UserRegisterDTO userRegisterDTO);
}
