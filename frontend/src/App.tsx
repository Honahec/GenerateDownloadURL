import {
  AlertDialog,
  AlertDialogBody,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogOverlay,
  Badge,
  Box,
  Button,
  Container,
  Divider,
  Flex,
  FormControl,
  FormLabel,
  Heading,
  HStack,
  IconButton,
  Input,
  Link,
  NumberInput,
  NumberInputField,
  Spacer,
  Stack,
  Switch,
  Text,
  VStack,
  useDisclosure,
  useToast,
} from "@chakra-ui/react";
import { CopyIcon, DeleteIcon, RepeatIcon } from "@chakra-ui/icons";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import axios from "axios";
import type {
  CreateLinkRequest,
  CreateLinkResponse,
  DownloadLinkResponse,
  ListLinksResponse,
  LoginResponse,
} from "./types";
import { API_CONFIG } from "./config";

const TOKEN_STORAGE_KEY = "signed-download-token";
const TOKEN_EXPIRY_STORAGE_KEY = "signed-download-token-exp";

interface LoginFormState {
  username: string;
  password: string;
}

interface LinkFormState {
  bucket: string;
  objectKey: string;
  expiresInMinutes: number;
  maxDownloads?: number;
  downloadFilename: string;
}

const initialLoginState: LoginFormState = {
  username: "",
  password: "",
};

const initialLinkFormState: LinkFormState = {
  bucket: "",
  objectKey: "",
  expiresInMinutes: 60,
  maxDownloads: undefined,
  downloadFilename: "",
};

const App = () => {
  const toast = useToast();
  const { isOpen, onOpen, onClose } = useDisclosure();
  const cancelRef = React.useRef<HTMLButtonElement>(null);
  const [authToken, setAuthToken] = useState<string | null>(null);
  const [loginForm, setLoginForm] = useState<LoginFormState>(initialLoginState);
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [linkForm, setLinkForm] = useState<LinkFormState>(initialLinkFormState);
  const [enforceLimit, setEnforceLimit] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [links, setLinks] = useState<CreateLinkResponse[]>([]);
  const [historyLinks, setHistoryLinks] = useState<DownloadLinkResponse[]>([]);
  const [linkToDelete, setLinkToDelete] = useState<DownloadLinkResponse | null>(
    null
  );

  useEffect(() => {
    const storedToken = localStorage.getItem(TOKEN_STORAGE_KEY);
    const storedExpiry = localStorage.getItem(TOKEN_EXPIRY_STORAGE_KEY);
    if (storedToken && storedExpiry) {
      const expiresAt = Number(storedExpiry);
      if (Number.isFinite(expiresAt) && expiresAt > Date.now()) {
        setAuthToken(storedToken);
      } else {
        localStorage.removeItem(TOKEN_STORAGE_KEY);
        localStorage.removeItem(TOKEN_EXPIRY_STORAGE_KEY);
      }
    }
  }, []);

  const axiosInstance = useMemo(() => {
    const instance = axios.create({
      baseURL: API_CONFIG.BASE_URL,
    });

    instance.interceptors.request.use((config) => {
      if (authToken) {
        config.headers.Authorization = `Bearer ${authToken}`;
      }
      return config;
    });

    return instance;
  }, [authToken]);

  const fetchHistoryLinks = useCallback(async () => {
    if (!authToken) return;
    try {
      const response = await axiosInstance.get<ListLinksResponse>("/links");
      setHistoryLinks(response.data.links);
    } catch (error) {
      console.error("Failed to fetch history links:", error);
    }
  }, [axiosInstance, authToken]);

  useEffect(() => {
    if (authToken) {
      fetchHistoryLinks();
    }
  }, [authToken, fetchHistoryLinks]);

  const handleLogin = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setIsLoggingIn(true);
      try {
        const response = await axios.post<LoginResponse>(
          `${API_CONFIG.BASE_URL}/login`,
          loginForm
        );
        setAuthToken(response.data.token);
        setLoginForm(initialLoginState);
        const expiresAt = Date.now() + response.data.expires_in * 1000;
        localStorage.setItem(TOKEN_STORAGE_KEY, response.data.token);
        localStorage.setItem(TOKEN_EXPIRY_STORAGE_KEY, String(expiresAt));
        toast({
          title: "登录成功",
          status: "success",
          duration: 2500,
          isClosable: true,
        });
      } catch (error) {
        console.error(error);
        toast({
          title: "登录失败",
          description: "请检查用户名和密码是否正确。",
          status: "error",
          duration: 3000,
          isClosable: true,
        });
      } finally {
        setIsLoggingIn(false);
      }
    },
    [loginForm, toast]
  );

  const handleLogout = useCallback(() => {
    setAuthToken(null);
    localStorage.removeItem(TOKEN_STORAGE_KEY);
    localStorage.removeItem(TOKEN_EXPIRY_STORAGE_KEY);
    toast({
      title: "已退出登录",
      status: "info",
      duration: 2000,
      isClosable: true,
    });
  }, [toast]);

  const handleCreateLink = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      if (!authToken) {
        toast({
          title: "需要登录",
          description: "请先登录后再生成下载链接。",
          status: "warning",
        });
        return;
      }

      if (!linkForm.objectKey.trim()) {
        toast({
          title: "对象键不能为空",
          status: "warning",
        });
        return;
      }

      setIsSubmitting(true);
      try {
        const payload: CreateLinkRequest = {
          object_key: linkForm.objectKey.trim(),
          expires_in_seconds: Math.round(linkForm.expiresInMinutes * 60),
        };
        if (linkForm.bucket.trim()) {
          payload.bucket = linkForm.bucket.trim();
        }
        if (linkForm.downloadFilename.trim()) {
          payload.download_filename = linkForm.downloadFilename.trim();
        }
        if (enforceLimit) {
          if (!linkForm.maxDownloads || linkForm.maxDownloads <= 0) {
            toast({
              title: "最大下载次数必须大于 0",
              status: "warning",
            });
            setIsSubmitting(false);
            return;
          }
          payload.max_downloads = linkForm.maxDownloads;
        }

        const response = await axiosInstance.post<CreateLinkResponse>(
          "/sign",
          payload
        );
        setLinks((prev) => [response.data, ...prev]);
        // 刷新历史链接列表
        fetchHistoryLinks();
        toast({
          title: "下载链接生成成功",
          status: "success",
          duration: 2500,
          isClosable: true,
        });
      } catch (error) {
        console.error(error);
        toast({
          title: "生成失败",
          description: "请检查填写信息，稍后重试。",
          status: "error",
          duration: 3000,
          isClosable: true,
        });
      } finally {
        setIsSubmitting(false);
      }
    },
    [authToken, enforceLimit, linkForm, axiosInstance, toast]
  );

  const resetLinkForm = useCallback(() => {
    setLinkForm(initialLinkFormState);
    setEnforceLimit(false);
  }, []);

  const copyToClipboard = useCallback(
    async (value: string) => {
      try {
        await navigator.clipboard.writeText(value);
        toast({ title: "已复制到剪贴板", status: "success", duration: 1500 });
      } catch (error) {
        console.error(error);
        toast({ title: "复制失败", status: "error", duration: 2000 });
      }
    },
    [toast]
  );

  const handleDeleteClick = useCallback(
    (link: DownloadLinkResponse) => {
      setLinkToDelete(link);
      onOpen();
    },
    [onOpen]
  );

  const confirmDelete = useCallback(async () => {
    if (!linkToDelete) return;

    try {
      await axiosInstance.delete(`/links/${linkToDelete.id}`);
      toast({
        title: "链接已删除",
        status: "success",
        duration: 2000,
        isClosable: true,
      });
      // 刷新历史链接列表
      fetchHistoryLinks();
    } catch (error) {
      console.error(error);
      toast({
        title: "删除失败",
        description: "请稍后重试",
        status: "error",
        duration: 3000,
        isClosable: true,
      });
    } finally {
      onClose();
      setLinkToDelete(null);
    }
  }, [axiosInstance, toast, fetchHistoryLinks, linkToDelete, onClose]);

  const renderLoginForm = () => (
    <Box
      as="form"
      onSubmit={handleLogin}
      bg="white"
      boxShadow="md"
      borderRadius="lg"
      p={8}
      w="100%"
    >
      <Heading size="md" mb={6} textAlign="center">
        管理员登录
      </Heading>
      <VStack spacing={4} align="stretch">
        <FormControl isRequired>
          <FormLabel>用户名</FormLabel>
          <Input
            value={loginForm.username}
            onChange={(e) =>
              setLoginForm((prev) => ({ ...prev, username: e.target.value }))
            }
            placeholder="请输入管理员用户名"
            autoComplete="username"
          />
        </FormControl>
        <FormControl isRequired>
          <FormLabel>密码</FormLabel>
          <Input
            type="password"
            value={loginForm.password}
            onChange={(e) =>
              setLoginForm((prev) => ({ ...prev, password: e.target.value }))
            }
            placeholder="请输入管理员密码"
            autoComplete="current-password"
          />
        </FormControl>
        <Button colorScheme="blue" type="submit" isLoading={isLoggingIn}>
          登录
        </Button>
      </VStack>
    </Box>
  );

  const renderLinkForm = () => (
    <Box
      as="form"
      onSubmit={handleCreateLink}
      bg="white"
      boxShadow="md"
      borderRadius="lg"
      p={8}
    >
      <Flex align="center" mb={6}>
        <Heading size="md">生成下载链接</Heading>
        <Spacer />
        <Button size="sm" variant="outline" onClick={handleLogout}>
          退出登录
        </Button>
      </Flex>
      <VStack spacing={5} align="stretch">
        <FormControl>
          <FormLabel>Bucket（可选）</FormLabel>
          <Input
            value={linkForm.bucket}
            onChange={(e) =>
              setLinkForm((prev) => ({ ...prev, bucket: e.target.value }))
            }
            placeholder="可选，使用后端默认 bucket"
          />
        </FormControl>
        <FormControl isRequired>
          <FormLabel>对象键（Object Key）</FormLabel>
          <Input
            value={linkForm.objectKey}
            onChange={(e) =>
              setLinkForm((prev) => ({ ...prev, objectKey: e.target.value }))
            }
            placeholder="示例：path/to/file.pdf"
          />
        </FormControl>
        <FormControl>
          <FormLabel>下载文件名（可选）</FormLabel>
          <Input
            value={linkForm.downloadFilename}
            onChange={(e) =>
              setLinkForm((prev) => ({
                ...prev,
                downloadFilename: e.target.value,
              }))
            }
            placeholder="下载时显示的文件名"
          />
        </FormControl>
        <FormControl>
          <FormLabel>有效期（分钟）</FormLabel>
          <NumberInput
            min={1}
            value={linkForm.expiresInMinutes}
            onChange={(_, value) =>
              setLinkForm((prev) => ({
                ...prev,
                expiresInMinutes: Number.isNaN(value) ? 60 : value,
              }))
            }
          >
            <NumberInputField />
          </NumberInput>
        </FormControl>
        <FormControl display="flex" alignItems="center">
          <HStack spacing={3}>
            <Switch
              isChecked={enforceLimit}
              onChange={(e) => setEnforceLimit(e.target.checked)}
            />
            <FormLabel m={0}>限制下载次数</FormLabel>
            <NumberInput
              min={1}
              value={linkForm.maxDownloads ?? ""}
              onChange={(_, value) =>
                setLinkForm((prev) => ({
                  ...prev,
                  maxDownloads: Number.isNaN(value) ? undefined : value,
                }))
              }
              isDisabled={!enforceLimit}
              w="120px"
            >
              <NumberInputField placeholder="默认10次" />
            </NumberInput>
          </HStack>
        </FormControl>
        <HStack spacing={4}>
          <Button colorScheme="blue" type="submit" isLoading={isSubmitting}>
            生成链接
          </Button>
          <Button
            leftIcon={<RepeatIcon />}
            variant="ghost"
            onClick={resetLinkForm}
          >
            重置
          </Button>
        </HStack>
      </VStack>
    </Box>
  );

  const renderLinkList = () => (
    <Box bg="white" boxShadow="md" borderRadius="lg" p={6}>
      <HStack spacing={4} mb={4} align="center">
        <Heading size="md">历史链接</Heading>
        <IconButton
          aria-label="刷新列表"
          icon={<RepeatIcon />}
          onClick={fetchHistoryLinks}
          variant="ghost"
          size="sm"
        />
      </HStack>
      {historyLinks.length === 0 ? (
        <Text color="gray.500">还没有生成链接。</Text>
      ) : (
        <Stack spacing={4} divider={<Divider />}>
          {historyLinks.map((link) => (
            <Box key={link.id}>
              <HStack spacing={3} align="flex-start">
                <Box flex="1">
                  <Text
                    fontWeight="semibold"
                    color={link.is_expired ? "gray.400" : "black"}
                  >
                    {link.object_key}
                  </Text>
                  <Text fontSize="sm" color="gray.600">
                    {link.download_filename &&
                      `文件名：${link.download_filename}`}
                  </Text>
                  <Text fontSize="sm" color="gray.600">
                    过期时间：{new Date(link.expires_at).toLocaleString()}
                  </Text>
                  <Text fontSize="sm" color="gray.600">
                    创建时间：{new Date(link.created_at).toLocaleString()}
                  </Text>
                  <HStack spacing={2} mt={2}>
                    {/* 状态标签 */}
                    {link.is_expired ? (
                      <Badge colorScheme="red">已过期</Badge>
                    ) : (
                      <Badge colorScheme="green">有效</Badge>
                    )}

                    {/* 使用次数标签 */}
                    {link.max_downloads ? (
                      <Badge
                        colorScheme={
                          link.downloads_served >= link.max_downloads
                            ? "red"
                            : "blue"
                        }
                      >
                        已使用 {link.downloads_served}/{link.max_downloads} 次
                      </Badge>
                    ) : (
                      <Badge colorScheme="purple">
                        已使用 {link.downloads_served} 次 (无限制)
                      </Badge>
                    )}
                  </HStack>
                </Box>
                <IconButton
                  aria-label="复制链接"
                  icon={<CopyIcon />}
                  onClick={() => copyToClipboard(link.download_url)}
                  variant="ghost"
                  isDisabled={
                    link.is_expired ||
                    (!!link.max_downloads &&
                      link.downloads_served >= link.max_downloads)
                  }
                />
                <Link
                  href={link.download_url}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <Button
                    variant="outline"
                    size="sm"
                    isDisabled={
                      link.is_expired ||
                      (!!link.max_downloads &&
                        link.downloads_served >= link.max_downloads)
                    }
                  >
                    下载
                  </Button>
                </Link>
                <IconButton
                  aria-label="删除链接"
                  icon={<DeleteIcon />}
                  onClick={() => handleDeleteClick(link)}
                  variant="ghost"
                  colorScheme="red"
                  size="sm"
                />
              </HStack>
            </Box>
          ))}
        </Stack>
      )}
    </Box>
  );

  return (
    <Box bg="gray.50" minH="100vh" py={10}>
      <Container maxW="4xl">
        <VStack spacing={8} align="stretch">
          <Heading textAlign="center">OSS Signed URL Generator</Heading>
          {!authToken ? (
            renderLoginForm()
          ) : (
            <>
              {renderLinkForm()}
              {renderLinkList()}
            </>
          )}
          <Box textAlign="center" color="gray.500" fontSize="sm">
            <VStack spacing={2}>
              <Text>
                <Link
                  href="https://beian.miit.gov.cn/"
                  target="_blank"
                  color="blue.500"
                >
                  鲁ICP备2024119517号-1
                </Link>
              </Text>
              <Text>
                Copyright © 2025
                {new Date().getFullYear() > 2025
                  ? `-${new Date().getFullYear()}`
                  : ""}{" "}
                Honahec
              </Text>
            </VStack>
          </Box>
        </VStack>
      </Container>

      {/* 确认删除对话框 */}
      <AlertDialog
        isOpen={isOpen}
        leastDestructiveRef={cancelRef}
        onClose={onClose}
      >
        <AlertDialogOverlay>
          <AlertDialogContent>
            <AlertDialogHeader fontSize="lg" fontWeight="bold">
              删除链接
            </AlertDialogHeader>
            <AlertDialogBody>
              确定要删除此下载链接吗？
              {linkToDelete && (
                <Box mt={2} p={2} bg="gray.50" borderRadius="md">
                  <Text fontSize="sm" fontWeight="semibold">
                    {linkToDelete.object_key}
                  </Text>
                  {linkToDelete.download_filename && (
                    <Text fontSize="xs" color="gray.600">
                      文件名：{linkToDelete.download_filename}
                    </Text>
                  )}
                </Box>
              )}
              <Text mt={2} color="red.500">
                此操作不可撤销。
              </Text>
            </AlertDialogBody>
            <AlertDialogFooter>
              <Button ref={cancelRef} onClick={onClose}>
                取消
              </Button>
              <Button colorScheme="red" onClick={confirmDelete} ml={3}>
                删除
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </Box>
  );
};

export default App;
