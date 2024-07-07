# Geektime Rust 语言训练营

## ecosystem 作业
1. 这周讲过的 crates，至少挑3个，
clone 下来，研究和运行其 examples
2. 在充分学习视频之后，不看已写好的代码，重写以下内容：
    - 聊天服务器（4-10）
    - url shortener（4-12）
3 (optional). 对于重写的 url shortener 重构并添加功能：
    - 使用 thiserror 进行错误处理（为你定义的 error 实现 IntoResponse）
    - 如果生成的 id 重复，而产生数据库错误，则重新生成一个 id，
      直到不再出错