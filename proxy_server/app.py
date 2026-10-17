import hmac
import os

from fastapi import FastAPI, Header, HTTPException

app = FastAPI()

@app.get("/health")
def health():
    return {"ok": True}

@app.get("/waey-provider")
def waey_provider(authorization: str | None = Header(default=None)):
    preset_token = os.getenv("WAEY_PRESET_TOKEN")
    groq_api_key = os.getenv("GROQ_API_KEY")

    if not preset_token or not groq_api_key:
        raise HTTPException(status_code=503, detail="Managed provider is not configured.")

    expected_authorization = f"Bearer {preset_token}"
    if not authorization or not hmac.compare_digest(authorization, expected_authorization):
        raise HTTPException(status_code=401, detail="Unauthorized managed provider request.")

    return {
        "id": "waey-managed-groq",
        "name": "Waey",
        "kind": "custom",
        "baseUrl": "https://api.groq.com/openai/v1",
        "model": "qwen/qwen3.8-27b",
        "apiKey": groq_api_key,
        "managed": True,
        "supportsVision": True,
        "revision": 3,
        "message": "Waey now uses the current Groq Qwen model. This update affects only the managed Waey provider and never changes providers you added yourself."
    }
