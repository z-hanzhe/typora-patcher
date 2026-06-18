# typora-patcher

Typora 激活工具，仅适用于`1.12.4`版本：

1. 正常安装 [typora-setup-x64-1.12.4.exe](https://github.com/z-hanzhe/typora-patcher/releases/download/v0.0.1/typora-setup-x64-1.12.4.exe) 并打开软件，然后在任务栏中右键关闭程序
2. 双击运行 [typora-patcher.exe](https://github.com/z-hanzhe/typora-patcher/releases/download/v0.0.1/typora-patcher.exe) 会打开一个命令行黑窗口，按照提示输入软件安装目录等信息
3. 再次打开 typora 选择离线激活，复制机器码到命令行黑窗口中进行激活
4. 如果输入激活码报错 app.asar 等字样，手动删除 typora 安装目录下的 resources 文件夹后重新激活即可
5. 激活后关闭 typora 的自动更新检查

![files/example.png](files/example.png)

> 原版JS激活工具

此工具非我原创，前段时间在网上找到 JS 版本的 Typora 激活工具`crack-minimal-fix.js`（仓库中有提供JS源码，可惜找不到原作者），该文件需要本机安装 node npm 才能执行并不是很方便，尤其对不懂编程的人很不友好，所以我拷打 AI 改了一份 Rust 版本，可编译为 exe 文件直接双击执行

如果 exe 由于某些原因无法执行，可尝试使用原本 JS 脚本进行激活：

1. 正常安装 typora-setup-x64-1.12.4.exe，打开软件后在任务栏中关闭程序
2. 在 typora 安装目录新建 activate 目录，将 crack-minimal-fix.js 放进去
3. 将 crack-minimal-fix.js 第 243 行位置，将原本的地址更换为 typora 安装目录地址
4. 在 activate 依次执行以下命令：
   ```shell
   npm init -y
   npm install asar chalk@4 readline-sync winreg @electron/fuses
   node crack-minimal-fix.js
   ```
5. 再次打开 typora 选择离线激活，复制机器码到 cli 中进行激活
6. 如果输入激活码报错 app.asar 等字样，手动删除 typora 安装目录下的 resources 文件夹后重新激活即可
7. 激活后关闭 typora 的自动更新检查
