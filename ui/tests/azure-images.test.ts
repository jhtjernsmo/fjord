import { describe, expect, test } from 'vitest'
import { azureAttachmentId, localCopies } from '../src/azureImages'

const GUID = '3F2504E0-4F89-11D3-9A0C-0305E82C3301'

describe('azure images', () => {
  test('finds the attachment id in an Azure image URL', () => {
    expect(azureAttachmentId(`https://dev.azure.com/contoso/_apis/wit/attachments/${GUID}?fileName=a.png`)).toBe(GUID.toLowerCase())
    expect(azureAttachmentId('https://example.com/cat.png')).toBeNull()
    expect(azureAttachmentId(undefined)).toBeNull()
  })

  test('maps ids to attachments that are still there', () => {
    const map = localCopies([
      { external_id: 'AA11', attachment_id: 4 },
      { external_id: 'bb22', attachment_id: null },
    ])
    expect([...map]).toEqual([['aa11', 4]])
  })
})
