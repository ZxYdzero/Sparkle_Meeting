package main

import (
	"fmt"

	"webrtc/room"
	"webrtc/router"

	"github.com/gin-gonic/gin"
)

func main() {
	gin.SetMode(gin.ReleaseMode)
	// 初始化房间管理器
	roomMgr := room.NewManager()

	// 创建路由器
	r := gin.Default()

	// 注册路由
	rt := router.NewRouter(roomMgr)
	rt.RegisterRoutes(&r.RouterGroup)

	// 静态文件服务
	//r.NoRoute(func(c *gin.Context) {
	//	c.FileFromFS(c.Request.URL.Path, http.Dir("./web"))
	//})
	r.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "POST, GET, OPTIONS, PUT, DELETE")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type, Content-Length, Accept-Encoding, X-CSRF-Token, Authorization")

		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(204)
			return
		}

		c.Next()
	})
	fmt.Printf("Server started on :9090\n")

	r.Run(":9090")
	// r.RunTLS(":9090", "./server.crt", "./server.key")
}
