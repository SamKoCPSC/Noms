'use client'
import React from "react";
import { Swiper, SwiperSlide } from "swiper/react";
import "swiper/css";
import "swiper/css/navigation";
import "swiper/css/pagination";
import { Navigation, Pagination } from "swiper/modules";
import { Box, Typography } from "@mui/material";
import theme from "../theme";

const Carousel = ({slides, slidesPerView, height}) => { 
    return (
      <Box sx={{ width: "100%", margin: "auto" }}>
        <Swiper
          modules={[Navigation, Pagination]}
          navigation
          pagination={{ clickable: true }}
          spaceBetween={30}
          slidesPerView={slidesPerView}
          loop
          style={{
            "--swiper-navigation-color": theme.palette.primary.main,
            "--swiper-navigation-size": "20px",
            "--swiper-pagination-color": theme.palette.primary.main,
            "--swiper-pagination-bullet-inactive-color": "#cfcfcf",
            "--swiper-pagination-bullet-size": "8px",
            "--swiper-pagination-bullet-horizontal-gap": "4px",
          }}
        >
          {slides.map((slide, index) => (
            <SwiperSlide key={index}>
              <Box
                sx={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  justifyContent: "center",
                  height: height
                }}
              >
                {slide}
              </Box>
            </SwiperSlide>
          ))}
        </Swiper>
      </Box>
    );
  };
  
  export default Carousel;