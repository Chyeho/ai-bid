package com.ithsd.smart_tender;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.scheduling.annotation.EnableAsync;
import org.springframework.scheduling.annotation.EnableScheduling;

@EnableAsync
@EnableScheduling
@SpringBootApplication
public class SmartTenderApplication {

    public static void main(String[] args) {
        SpringApplication.run(SmartTenderApplication.class, args);
    }

}
