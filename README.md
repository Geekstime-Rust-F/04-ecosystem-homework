# Geektime Rust 语言训练营

## ecosystem 作业
1. 这周讲过的 crates，至少挑3个，
clone 下来，研究和运行其 examples
2. 在充分学习视频之后，不看已写好的代码，重写以下内容：
    - 聊天服务器（4-10）
        - client 连接：添加到全局状态
            - 创建 peer
            - 通知所有小伙伴
        - client 断连：从全局状态删除
            - 通知所有小伙伴
        - client 发消息
            - 广播
    - url shortener（4-12）
        要求暴露两个api：
        - POST http:localhost:9876/
            - request body
            ```json
            {
                "url": "http://www.google.com"
            }
            ```
            - response body
            ```json
            {
                "url": "http://localhost/abc123"
            }
            ```
        - GET http:localhost:9876/abc123
            要求redirect到 http://www.google.com
        tasks:
        - 暴露以上两个api
        - 生成短链接的算法
        - 保存短链接和原链接的映射
        - 如果已经存在的url再次post, 返回已有的短链接
3 (optional). 对于重写的 url shortener 重构并添加功能：
    - 使用 thiserror 进行错误处理（为你定义的 error 实现 IntoResponse）
    - 如果生成的 id 重复，而产生数据库错误，则重新生成一个 id，
      直到不再出错
